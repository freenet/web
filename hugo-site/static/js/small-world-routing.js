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
    const canvas = document.getElementById('networkCanvas2');
    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Parameters
    const numPeers = 50;
    const radius = 200;
    const connectionProbability = (distance) => 1 / (distance + 1);

    let peers = [];
    let links = [];
    let currentPath = [];
    let animationFrame;
    let sourceNode, targetNode;

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
            // Always connect adjacent peers
            const nextIndex = (i + 1) % numPeers;
            links.push({ source: peers[i], target: peers[nextIndex] });
            
            // Additional links based on distance and probability
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

        // Draw path
        if (currentPath.length > 1) {
            ctx.strokeStyle = 'rgba(255, 0, 0, 0.7)';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(currentPath[0].x, currentPath[0].y);
            for (let i = 1; i < currentPath.length; i++) {
                ctx.lineTo(currentPath[i].x, currentPath[i].y);
            }
            ctx.stroke();
            ctx.lineWidth = 1;
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

    function findPath(source, target) {
        const path = [];
        const visited = new Set();
        let currentNode = source;
        path.push(currentNode);
        visited.add(currentNode.index);

        while (currentNode !== target) {
            let closestNeighbor = null;
            let closestDistance = Infinity;

            links.forEach(link => {
                let neighbor = null;
                if (link.source.index === currentNode.index) {
                    neighbor = link.target;
                } else if (link.target.index === currentNode.index) {
                    neighbor = link.source;
                }

                if (neighbor && !visited.has(neighbor.index)) {
                    const distance = Math.hypot(neighbor.x - target.x, neighbor.y - target.y);
                    if (distance < closestDistance) {
                        closestDistance = distance;
                        closestNeighbor = neighbor;
                    }
                }
            });

            if (!closestNeighbor) break;
            
            currentNode = closestNeighbor;
            path.push(currentNode);
            visited.add(currentNode.index);
            
            if (currentNode === target) break;
        }

        return path;
    }

    function animatePath() {
        const path = findPath(sourceNode, targetNode);
        let step = 1;

        function animate() {
            currentPath = path.slice(0, step);
            draw();
            
            if (step < path.length) {
                step++;
                animationFrame = requestAnimationFrame(animate);
            }
        }

        cancelAnimationFrame(animationFrame);
        animate();
    }

    function startNewRoute() {
        sourceNode = peers[Math.floor(Math.random() * peers.length)];
        do {
            targetNode = peers[Math.floor(Math.random() * peers.length)];
        } while (targetNode === sourceNode);
        
        currentPath = [];
        animatePath();
    }

    // Initialize
    initializeNetwork();
    draw();

    // Add button handler
    document.getElementById('newRouteBtn').addEventListener('click', startNewRoute);
    
    // Start first route
    startNewRoute();
});
