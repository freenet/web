// Wait for D3.js to be available
function waitForD3() {
    return new Promise((resolve) => {
        if (typeof d3 !== 'undefined') {
            resolve();
        } else {
            const script = document.createElement('script');
            script.src = "https://d3js.org/d3.v7.min.js";
            script.onload = () => resolve();
            document.head.appendChild(script);
        }
    });
}

waitForD3().then(() => {
    const canvas = document.getElementById('networkCanvas3');
    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Parameters
    let numPeers = 30;
    const maxPeers = 10000;
    const radius = Math.min(width, height) * 0.4;
    const connectionProbability = (distance) => 1 / (distance + 1);
    
    let peers = [];
    let links = [];
    let averagePathLengths = [];
    let animationFrame;

    function initializeNetwork() {
        peers = d3.range(numPeers).map(i => {
            const angle = (i / numPeers) * 2 * Math.PI;
            return {
                x: width / 2 + radius * Math.cos(angle),
                y: height / 2 + radius * Math.sin(angle),
                index: i
            };
        });

        links = [];
        for (let i = 0; i < numPeers; i++) {
            const nextIndex = (i + 1) % numPeers;
            links.push({ source: peers[i], target: peers[nextIndex] });
            
            for (let j = i + 2; j < numPeers; j++) {
                const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
                const prob = connectionProbability(distance);
                if (Math.random() < prob) {
                    links.push({ source: peers[i], target: peers[j] });
                }
            }
        }
    }

    function draw() {
        ctx.clearRect(0, 0, width, height);

        // Draw links
        ctx.strokeStyle = 'rgba(100, 149, 237, 0.3)';
        links.forEach(link => {
            ctx.beginPath();
            ctx.moveTo(link.source.x, link.source.y);
            ctx.lineTo(link.target.x, link.target.y);
            ctx.stroke();
        });

        // Draw nodes
        ctx.fillStyle = 'tomato';
        peers.forEach(peer => {
            ctx.beginPath();
            ctx.arc(peer.x, peer.y, 3, 0, 2 * Math.PI);
            ctx.fill();
        });
    }

    function calculateAveragePathLength() {
        let totalLength = 0;
        let pathCount = 0;
        
        // Sample only a subset of paths for larger networks
        const sampleSize = numPeers > 100 ? 200 : numPeers * 2;
        
        for (let k = 0; k < sampleSize; k++) {
            const i = Math.floor(Math.random() * peers.length);
            const j = Math.floor(Math.random() * peers.length);
            if (i !== j) {
                const path = findShortestPath(peers[i], peers[j]);
                if (path) {
                    totalLength += path.length - 1;
                    pathCount++;
                }
            }
        }

        return totalLength / pathCount;
    }

    function findShortestPath(start, end) {
        const queue = [[start]];
        const visited = new Set([start.index]);

        while (queue.length > 0) {
            const path = queue.shift();
            const node = path[path.length - 1];

            if (node === end) {
                return path;
            }

            // Find all neighbors
            const neighbors = links.reduce((acc, link) => {
                if (link.source === node && !visited.has(link.target.index)) {
                    acc.push(link.target);
                } else if (link.target === node && !visited.has(link.source.index)) {
                    acc.push(link.source);
                }
                return acc;
            }, []);

            for (const neighbor of neighbors) {
                visited.add(neighbor.index);
                queue.push([...path, neighbor]);
            }
        }

        return null;
    }

    function updateChart() {
        const margin = {top: 20, right: 20, bottom: 30, left: 40};
        const chartWidth = width - margin.left - margin.right;
        const chartHeight = height - margin.top - margin.bottom;

        const svg = d3.select('#scalingChart svg');
        svg.selectAll('*').remove();

        const g = svg.append('g')
            .attr('transform', `translate(${margin.left},${margin.top})`);

        const x = d3.scaleLinear()
            .domain([30, maxPeers])
            .range([0, chartWidth]);

        const y = d3.scaleLinear()
            .domain([0, d3.max(averagePathLengths, d => d.pathLength) * 1.1])
            .range([chartHeight, 0]);

        // Add the axes
        g.append('g')
            .attr('transform', `translate(0,${chartHeight})`)
            .call(d3.axisBottom(x));

        g.append('g')
            .call(d3.axisLeft(y));

        // Add axis labels
        g.append('text')
            .attr('transform', `translate(${chartWidth/2},${chartHeight + margin.bottom})`)
            .style('text-anchor', 'middle')
            .text('Network Size');

        g.append('text')
            .attr('transform', 'rotate(-90)')
            .attr('y', 0 - margin.left)
            .attr('x', 0 - (chartHeight / 2))
            .attr('dy', '1em')
            .style('text-anchor', 'middle')
            .text('Average Path Length');

        // Plot points
        g.selectAll('circle')
            .data(averagePathLengths)
            .enter()
            .append('circle')
            .attr('cx', d => x(d.numPeers))
            .attr('cy', d => y(d.pathLength))
            .attr('r', 3)
            .style('fill', 'steelblue');

        // Add connecting line
        const line = d3.line()
            .x(d => x(d.numPeers))
            .y(d => y(d.pathLength));

        g.append('path')
            .datum(averagePathLengths)
            .attr('fill', 'none')
            .attr('stroke', 'steelblue')
            .attr('stroke-width', 1.5)
            .attr('d', line);
    }

    // Create Web Worker
    const worker = new Worker('/js/network-worker.js');
    
    worker.onmessage = function(e) {
        const { peers: newPeers, links: newLinks, avgPathLength, numPeers: currentPeers } = e.data;
        
        peers = newPeers;
        links = newLinks;
        
        // Reduce drawing frequency as network grows
        const drawThreshold = Math.ceil(currentPeers / 100) * 30;
        if (currentPeers % drawThreshold === 0) {
            draw();
        }
        
        if (currentPeers % 100 === 0) {
            averagePathLengths.push({
                numPeers: currentPeers,
                pathLength: avgPathLength
            });
            updateChart();
        }
        
        if (isSimulating && currentPeers <= maxPeers) {
            // Continue simulation with next batch
            setTimeout(() => {
                worker.postMessage({ 
                    numPeers: currentPeers + 50,
                    maxPeers 
                });
            }, 200);
        } else {
            isSimulating = false;
            startBtn.textContent = '▶️ Start';
        }
    };

    function simulate() {
        if (!isSimulating) {
            isSimulating = true;
            startBtn.textContent = '⏸️ Pause';
            worker.postMessage({ numPeers, maxPeers });
        } else {
            isSimulating = false;
            startBtn.textContent = '▶️ Start';
        }
    }

    function reset() {
        cancelAnimationFrame(animationFrame);
        numPeers = 30;
        averagePathLengths = [];
        simulate();
    }

    // Initialize network but don't start simulation
    initializeNetwork();
    draw();
    
    const startBtn = document.getElementById('startScaleBtn');
    const resetBtn = document.getElementById('resetScaleBtn');
    
    // Add button handlers
    let isSimulating = false;
    let animationFrameId = null;
    
    startBtn.addEventListener('click', () => {
        if (!isSimulating) {
            // Resume/start simulation
            isSimulating = true;
            startBtn.textContent = '⏸️ Pause';
            simulate();
        } else {
            // Pause simulation
            isSimulating = false;
            startBtn.textContent = '▶️ Start';
            clearTimeout(animationFrameId);
        }
    });
    
    resetBtn.addEventListener('click', () => {
        // Reset everything
        isSimulating = false;
        startBtn.textContent = '▶️ Start';
        cancelAnimationFrame(animationFrameId);
        numPeers = 30;
        averagePathLengths = [];
        initializeNetwork();
        draw();
    });
});
