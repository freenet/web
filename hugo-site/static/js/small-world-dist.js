let isPlaying = false;
let animationFrame = 0;
const maxAnimationFrames = 100;

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

// Initialize after D3 is loaded
waitForD3().then(() => {

const canvas1 = document.getElementById('networkCanvas1');
const ctx1 = canvas1.getContext('2d');
const width1 = canvas1.width;
const height1 = canvas1.height;

// Parameters
const numPeers = 50;
const radius = 200;
const connectionProbability = (distance) => 1 / (distance + 1);

let peers = [];
let links = [];
let distances = [];

function togglePlayPause() {
    isPlaying = !isPlaying;
    const icon = document.querySelector('#distPlayPauseBtn i');
    icon.className = isPlaying ? 'fas fa-pause' : 'fas fa-play';
    if (isPlaying) {
        animate();
    }
}

function resetAnimation() {
    isPlaying = false;
    animationFrame = 0;
    const icon = document.querySelector('#distPlayPauseBtn i');
    icon.className = 'fas fa-play';
    initializeNetwork(true);
}

function animate() {
    if (!isPlaying) return;
    
    if (animationFrame < maxAnimationFrames) {
        animationFrame++;
        updateNetworkState();
        requestAnimationFrame(animate);
    } else {
        isPlaying = false;
        const icon = document.querySelector('#distPlayPauseBtn i');
        icon.className = 'fas fa-play';
    }
}

function updateNetworkState() {
    // Clear non-ring links
    links = links.filter((link, i) => {
        const sourceIdx = link.source.index;
        const targetIdx = link.target.index;
        const distance = Math.min(Math.abs(sourceIdx - targetIdx), 
                                numPeers - Math.abs(sourceIdx - targetIdx));
        return distance === 1;
    });
    distances = distances.filter(d => d === 1);
    
    // Add long-range links progressively
    const progress = animationFrame / maxAnimationFrames;
    
    for (let i = 0; i < numPeers; i++) {
        for (let j = i + 2; j < numPeers; j++) {
            const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
            const prob = connectionProbability(distance) * progress;
            if (Math.random() < prob) {
                links.push({ source: peers[i], target: peers[j] });
                distances.push(distance);
            }
        }
    }
    
    drawNetwork();
    updateHistogram();
}

function initializeNetwork(initialStateOnly = false) {
    // Calculate positions for peers on a 1D ring
    peers = d3.range(numPeers).map(i => {
        const angle = (i / numPeers) * 2 * Math.PI;
        return {
            x: width1 / 2 + radius * Math.cos(angle),
            y: height1 / 2 + radius * Math.sin(angle),
            index: i
        };
    });

    // Initialize with only ring connections
    links = [];
    distances = [];
    
    // Always add ring connections
    for (let i = 0; i < numPeers; i++) {
        const nextIndex = (i + 1) % numPeers;
        links.push({ source: peers[i], target: peers[nextIndex] });
        distances.push(1);
    }
    
    // Add other connections only if not in initial state
    if (!initialStateOnly) {
        for (let i = 0; i < numPeers; i++) {
            for (let j = i + 2; j < numPeers; j++) {
                const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
                const prob = connectionProbability(distance);
                if (Math.random() < prob) {
                    links.push({ source: peers[i], target: peers[j] });
                    distances.push(distance);
                }
            }
        }
    }

    drawNetwork();
    updateHistogram();
}

function drawNetwork() {
    ctx1.clearRect(0, 0, width1, height1);

    // Draw links
    ctx1.strokeStyle = 'rgba(0, 127, 255, 0.3)'; // Using website link color
    links.forEach(link => {
        ctx1.beginPath();
        ctx1.moveTo(link.source.x, link.source.y);
        ctx1.lineTo(link.target.x, link.target.y);
        ctx1.stroke();
    });

    // Draw nodes
    ctx1.fillStyle = '#007FFF'; // Primary blue
    peers.forEach(peer => {
        ctx1.beginPath();
        ctx1.arc(peer.x, peer.y, 5, 0, 2 * Math.PI);
        ctx1.fill();
    });
}

function updateHistogram() {
    const histogramDiv = document.getElementById('histogram');
    const margin = {top: 20, right: 20, bottom: 30, left: 40};
    const width = histogramDiv.clientWidth - margin.left - margin.right;
    const height = histogramDiv.clientHeight - margin.top - margin.bottom;

    // Clear previous histogram
    d3.select('#histogram').selectAll('*').remove();

    const svg = d3.select('#histogram')
        .append('svg')
        .attr('width', width + margin.left + margin.right)
        .attr('height', height + margin.top + margin.bottom)
        .append('g')
        .attr('transform', `translate(${margin.left},${margin.top})`);

    // Create histogram data with more bins for smoother curve
    const bins = d3.histogram()
        .domain([0, Math.floor(numPeers/2)])
        .thresholds(20)
        (distances);

    // Apply square root transformation to y-values for better visualization
    const yScale = d => Math.sqrt(d);

    // Scales
    const x = d3.scaleLinear()
        .domain([0, Math.floor(numPeers/2)])
        .range([0, width]);

    const y = d3.scaleLinear()
        .domain([0, Math.sqrt(d3.max(bins, d => d.length))])
        .range([height, 0]);

    // Add bars
    svg.selectAll('rect')
        .data(bins)
        .enter()
        .append('rect')
        .attr('x', d => x(d.x0))
        .attr('width', d => Math.max(0, x(d.x1) - x(d.x0) - 1))
        .attr('y', d => y(yScale(d.length)))
        .attr('height', d => height - y(yScale(d.length)))
        .style('fill', '#007FFF'); // Primary blue

    // Add axes
    svg.append('g')
        .attr('transform', `translate(0,${height})`)
        .call(d3.axisBottom(x)
            .ticks(5)
            .tickFormat(d => Math.round(d)));

    svg.append('g')
        .call(d3.axisLeft(y));

    // Add labels
    svg.append('text')
        .attr('x', width/2)
        .attr('y', height + margin.bottom)
        .style('text-anchor', 'middle')
        .text('Link Distance');

    svg.append('text')
        .attr('transform', 'rotate(-90)')
        .attr('x', -height/2)
        .attr('y', -margin.left)
        .style('text-anchor', 'middle')
        .text('Frequency');
}

    // Add button event listeners
    document.getElementById('distPlayPauseBtn').addEventListener('click', togglePlayPause);
    document.getElementById('resetDistBtn').addEventListener('click', resetAnimation);
    
    // Initialize with only ring connections
    initializeNetwork(true);

    // Add resize handler
    window.addEventListener('resize', () => {
        updateHistogram();
    });
});
