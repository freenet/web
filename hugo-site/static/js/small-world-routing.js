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

// Initialize visualization
async function initVisualization() {
    try {
        // Wait for both D3 and DOM to be ready
        await waitForD3();
        
        // Get canvas element
        const canvas = document.getElementById('networkCanvas2');
        if (!canvas) {
            console.error('Canvas element not found');
            return;
        }
        
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
    let animationProgress = 0;
    let currentPathSegment = 0;

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

        // Draw completed path segments
        if (currentPath.length > 1) {
            ctx.strokeStyle = 'rgba(255, 0, 0, 0.4)';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(currentPath[0].x, currentPath[0].y);
            for (let i = 1; i <= currentPathSegment; i++) {
                ctx.lineTo(currentPath[i].x, currentPath[i].y);
            }
            ctx.stroke();
            ctx.lineWidth = 1;
        }

        // Draw animated request
        if (currentPath.length > 1 && currentPathSegment < currentPath.length - 1) {
            const start = currentPath[currentPathSegment];
            const end = currentPath[currentPathSegment + 1];
            
            // Draw moving request dot
            const x = start.x + (end.x - start.x) * animationProgress;
            const y = start.y + (end.y - start.y) * animationProgress;
            
            ctx.fillStyle = 'yellow';
            ctx.strokeStyle = 'black';
            ctx.beginPath();
            ctx.arc(x, y, 8, 0, 2 * Math.PI);
            ctx.fill();
            ctx.stroke();
        }

        // Draw nodes
        peers.forEach(peer => {
            if (peer === sourceNode) {
                ctx.fillStyle = 'green';
                ctx.strokeStyle = 'black';
                ctx.lineWidth = 2;
            } else if (peer === targetNode) {
                ctx.fillStyle = 'red';
                ctx.strokeStyle = 'black';
                ctx.lineWidth = 2;
            } else {
                ctx.fillStyle = 'tomato';
                ctx.strokeStyle = 'rgba(0,0,0,0.2)';
                ctx.lineWidth = 1;
            }
            ctx.beginPath();
            ctx.arc(peer.x, peer.y, 5, 0, 2 * Math.PI);
            ctx.fill();
            ctx.stroke();
        });

        // Add labels for source and target
        ctx.font = '14px Arial';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'bottom';
        
        if (sourceNode) {
            ctx.fillStyle = 'black';
            ctx.fillText('Source', sourceNode.x, sourceNode.y - 10);
        }
        if (targetNode) {
            ctx.fillStyle = 'black';
            ctx.fillText('Target', targetNode.x, targetNode.y - 10);
        }
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
        if (!isPlaying) return;
        
        const path = findPath(sourceNode, targetNode);
        currentPath = path;
        currentPathSegment = 0;
        animationProgress = 0;

        function animate() {
            if (!isPlaying) return;
            
            animationProgress += 0.02;
            
            if (animationProgress >= 1) {
                animationProgress = 0;
                currentPathSegment++;
                
                if (currentPathSegment >= currentPath.length - 1) {
                    currentPathSegment = 0;
                    // Restart animation after a delay
                    setTimeout(() => {
                        if (!isPlaying) return;
                        animationProgress = 0;
                        currentPathSegment = 0;
                        animationFrame = requestAnimationFrame(animate);
                    }, 1000);
                    return;
                }
            }
            
            draw();
            animationFrame = requestAnimationFrame(animate);
        }

        if (animationFrame) {
            cancelAnimationFrame(animationFrame);
        }
        animate();
    }

    let isPlaying = false;
    let routeTimeout;

    function startNewRoute() {
        sourceNode = peers[Math.floor(Math.random() * peers.length)];
        do {
            targetNode = peers[Math.floor(Math.random() * peers.length)];
        } while (targetNode === sourceNode);
        
        currentPath = [];
        animatePath();

        if (isPlaying) {
            routeTimeout = setTimeout(startNewRoute, 3000); // Start new route every 3 seconds
        }
    }

    function togglePlayPause() {
        isPlaying = !isPlaying;
        const btn = document.getElementById('routingPlayPauseBtn');
        if (!btn) {
            console.error('Play/Pause button not found');
            return;
        }
        
        const icon = btn.querySelector('i');
        if (!icon) {
            console.error('Button icon not found');
            return;
        }
        
        if (isPlaying) {
            icon.className = 'fas fa-pause';
            startNewRoute();
        } else {
            icon.className = 'fas fa-play';
            clearTimeout(routeTimeout);
        }
    }

    // Initialize everything in sequence
    initializeNetwork();
    draw();
    
        // Debug: Log all elements with IDs containing 'play' or 'pause'
        const allElements = document.querySelectorAll('[id]');
        console.log('Available elements with IDs:', Array.from(allElements).map(el => ({
            id: el.id,
            tagName: el.tagName,
            classes: el.className
        })));

        // Setup button with detailed error checking
        const playPauseBtn = document.getElementById('routingPlayPauseBtn');
        console.log('Found button:', playPauseBtn);
        
        if (playPauseBtn) {
            console.log('Button details:', {
                id: playPauseBtn.id,
                tagName: playPauseBtn.tagName,
                classes: playPauseBtn.className,
                innerHTML: playPauseBtn.innerHTML
            });
            
            try {
                playPauseBtn.addEventListener('click', togglePlayPause);
                console.log('Successfully added click listener');
                isPlaying = false; // Ensure initial state
                togglePlayPause(); // Start the animation
            } catch (err) {
                console.error('Error adding event listener:', err);
                // Fallback to auto-play
                isPlaying = true;
                startNewRoute();
            }
        } else {
            console.error('Play/Pause button not found - searched for ID: routingPlayPauseBtn');
            // Start animation anyway
            isPlaying = true;
            startNewRoute();
        }
    } catch (error) {
        console.error('Failed to initialize visualization:', error);
        // Try to continue with basic functionality
        isPlaying = true;
        startNewRoute();
    }
}

// Wait for DOM to be fully loaded before starting
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initVisualization);
} else {
    initVisualization();
}
