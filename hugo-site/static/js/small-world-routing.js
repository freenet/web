// Network parameters
const width = 800;
const height = 800;
const radius = 300;
const numPeers = 30;
const connectionProbability = (distance) => 1 / (distance + 1);

// Initialize canvas
const canvas = document.getElementById('routingCanvas');
const ctx = canvas.getContext('2d');

let peers = [];
let links = [];
let currentPath = [];
let sourceNode = null;
let targetNode = null;
let animationFrame = null;

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

function drawNetwork() {
    ctx.clearRect(0, 0, width, height);

    // Draw links
    ctx.strokeStyle = 'rgba(100, 149, 237, 0.3)';
    ctx.lineWidth = 1;
    links.forEach(link => {
        ctx.beginPath();
        ctx.moveTo(link.source.x, link.source.y);
        ctx.lineTo(link.target.x, link.target.y);
        ctx.stroke();
    });

    // Draw routing path
    if (currentPath.length > 1) {
        ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(currentPath[0].x, currentPath[0].y);
        for (let i = 1; i < currentPath.length; i++) {
            ctx.lineTo(currentPath[i].x, currentPath[i].y);
        }
        ctx.stroke();
    }

    // Draw nodes
    peers.forEach(peer => {
        if (peer === sourceNode) {
            ctx.fillStyle = 'green';
        } else if (peer === targetNode) {
            ctx.fillStyle = 'red';
        } else {
            ctx.fillStyle = 'tomato';
        }
        ctx.beginPath();
        ctx.arc(peer.x, peer.y, 5, 0, 2 * Math.PI);
        ctx.fill();
    });
}

function findNextHop(current, target) {
    let bestNeighbor = null;
    let bestDistance = Infinity;

    links.forEach(link => {
        let neighbor = null;
        if (link.source === current) neighbor = link.target;
        if (link.target === current) neighbor = link.source;

        if (neighbor && !currentPath.includes(neighbor)) {
            const distance = Math.hypot(neighbor.x - target.x, neighbor.y - target.y);
            if (distance < bestDistance) {
                bestDistance = distance;
                bestNeighbor = neighbor;
            }
        }
    });

    return bestNeighbor;
}

function animateRouting() {
    if (!currentPath.length || currentPath[currentPath.length - 1] === targetNode) {
        cancelAnimationFrame(animationFrame);
        return;
    }

    const current = currentPath[currentPath.length - 1];
    const next = findNextHop(current, targetNode);

    if (next) {
        currentPath.push(next);
        drawNetwork();
    } else {
        cancelAnimationFrame(animationFrame);
        return;
    }

    animationFrame = requestAnimationFrame(animateRouting);
}

function startNewRoute() {
    // Cancel any ongoing animation
    cancelAnimationFrame(animationFrame);

    // Select random source and target
    sourceNode = peers[Math.floor(Math.random() * peers.length)];
    do {
        targetNode = peers[Math.floor(Math.random() * peers.length)];
    } while (sourceNode === targetNode);

    // Initialize path with source
    currentPath = [sourceNode];

    // Start animation
    animationFrame = requestAnimationFrame(animateRouting);
}

// Event listeners
document.getElementById('newRouteBtn').addEventListener('click', startNewRoute);

// Initial setup
initializeNetwork();
drawNetwork();
startNewRoute();
