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

function initializeNetwork() {
    // Calculate positions for peers on a 1D ring
    peers = d3.range(numPeers).map(i => {
        const angle = (i / numPeers) * 2 * Math.PI;
        return {
            x: width1 / 2 + radius * Math.cos(angle),
            y: height1 / 2 + radius * Math.sin(angle),
            index: i
        };
    });

    // Generate links and collect distances
    links = [];
    distances = [];
    
    for (let i = 0; i < numPeers; i++) {
        // Always connect adjacent peers
        const nextIndex = (i + 1) % numPeers;
        links.push({ source: peers[i], target: peers[nextIndex] });
        distances.push(1);
        
        // Additional links based on distance and probability
        for (let j = i + 2; j < numPeers; j++) {
            const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
            const prob = connectionProbability(distance);
            if (Math.random() < prob) {
                links.push({ source: peers[i], target: peers[j] });
                distances.push(distance);
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

    // Create histogram data
    const bins = d3.histogram()
        .domain([0, Math.floor(numPeers/2)])
        .thresholds(10)
        (distances);

    // Scales
    const x = d3.scaleLinear()
        .domain([0, Math.floor(numPeers/2)])
        .range([0, width]);

    const y = d3.scaleLinear()
        .domain([0, d3.max(bins, d => d.length)])
        .range([height, 0]);

    // Add bars
    svg.selectAll('rect')
        .data(bins)
        .enter()
        .append('rect')
        .attr('x', d => x(d.x0))
        .attr('width', d => Math.max(0, x(d.x1) - x(d.x0) - 1))
        .attr('y', d => y(d.length))
        .attr('height', d => height - y(d.length))
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

    // Initialize visualization
    initializeNetwork();

    // Add resize handler
    window.addEventListener('resize', () => {
        updateHistogram();
    });
});
