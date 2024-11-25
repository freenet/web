// Network parameters
const width = 800;
const height = 800;
const radius = 300;
const numPeers = 30;
const connectionProbability = (distance) => 1 / (distance + 1);

// Initialize canvas and D3
const canvas = document.getElementById('networkCanvas');
const ctx = canvas.getContext('2d');
const histogram = d3.select('#histogram');

let peers = [];
let links = [];

function initializeNetwork() {
    // Calculate positions for peers on a 1D ring
    peers = d3.range(numPeers).map(i => {
        const angle = (i / numPeers) * 2 * Math.PI;
        return {
            x: width / 2 + radius * Math.cos(angle),
            y: height / 2 + radius * Math.sin(angle),
            index: i
        };
    });

    // Generate links between peers
    links = [];
    for (let i = 0; i < numPeers; i++) {
        // Connect adjacent peers
        const nextIndex = (i + 1) % numPeers;
        links.push({ 
            source: peers[i], 
            target: peers[nextIndex],
            distance: 1
        });
        
        // Additional probabilistic links
        for (let j = i + 2; j < numPeers; j++) {
            const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
            const prob = connectionProbability(distance);
            if (Math.random() < prob) {
                links.push({ 
                    source: peers[i], 
                    target: peers[j],
                    distance: distance
                });
            }
        }
    }
}

function drawNetwork() {
    ctx.clearRect(0, 0, width, height);

    // Draw links with color based on distance
    links.forEach(link => {
        const color = d3.interpolateBlues(1 - link.distance / (numPeers / 2));
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(link.source.x, link.source.y);
        ctx.lineTo(link.target.x, link.target.y);
        ctx.stroke();
    });

    // Draw nodes
    peers.forEach(peer => {
        ctx.fillStyle = 'tomato';
        ctx.beginPath();
        ctx.arc(peer.x, peer.y, 5, 0, 2 * Math.PI);
        ctx.fill();
    });
}

function updateHistogram() {
    // Group links by distance
    const distanceCounts = d3.rollup(
        links,
        v => v.length,
        d => d.distance
    );

    // Convert to array for D3
    const data = Array.from(distanceCounts, ([distance, count]) => ({
        distance,
        count
    })).sort((a, b) => a.distance - b.distance);

    // Clear previous histogram
    histogram.selectAll('*').remove();

    // Create SVG
    const svg = histogram.append('svg')
        .attr('width', width)
        .attr('height', 200);

    // Create scales
    const x = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.distance)])
        .range([40, width - 40]);

    const y = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.count)])
        .range([160, 20]);

    // Draw bars
    svg.selectAll('rect')
        .data(data)
        .enter()
        .append('rect')
        .attr('x', d => x(d.distance))
        .attr('y', d => y(d.count))
        .attr('width', width / (numPeers / 2) - 2)
        .attr('height', d => 160 - y(d.count))
        .attr('fill', d => d3.interpolateBlues(1 - d.distance / (numPeers / 2)));

    // Add axes
    const xAxis = d3.axisBottom(x);
    const yAxis = d3.axisLeft(y);

    svg.append('g')
        .attr('transform', 'translate(0, 160)')
        .call(xAxis);

    svg.append('g')
        .attr('transform', 'translate(40, 0)')
        .call(yAxis);

    // Add labels
    svg.append('text')
        .attr('x', width / 2)
        .attr('y', 190)
        .attr('text-anchor', 'middle')
        .text('Link Distance');

    svg.append('text')
        .attr('transform', 'rotate(-90)')
        .attr('x', -80)
        .attr('y', 15)
        .attr('text-anchor', 'middle')
        .text('Number of Links');
}

function initialize() {
    initializeNetwork();
    drawNetwork();
    updateHistogram();
}

// Event listeners
document.getElementById('resetBtn').addEventListener('click', initialize);

// Initial setup
initialize();
