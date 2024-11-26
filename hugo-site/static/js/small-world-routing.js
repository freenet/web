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
    console.log('Starting visualization initialization');
    
    // Check if we're in a browser environment
    if (typeof window === 'undefined' || typeof document === 'undefined') {
        console.error('Not in browser environment');
        return;
    }

    try {
        console.log('Waiting for D3...');
        await waitForD3();
        console.log('D3 loaded successfully');
        
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
        ctx.strokeStyle = 'rgba(0, 127, 255, 0.3)'; // Using website link color
        links.forEach(link => {
            ctx.beginPath();
            ctx.moveTo(link.source.x, link.source.y);
            ctx.lineTo(link.target.x, link.target.y);
            ctx.stroke();
        });

        // Draw the complete path up to current segment
        if (currentPath.length > 1) {
            // Draw the completed segments
            ctx.strokeStyle = 'rgba(0, 127, 255, 0.6)'; // Using primary blue
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(currentPath[0].x, currentPath[0].y);
            for (let i = 1; i <= currentPathSegment + 1; i++) {
                ctx.lineTo(currentPath[i].x, currentPath[i].y);
            }
            ctx.stroke();
            ctx.lineWidth = 1;
        }

        // Draw the animated request dot
        if (currentPath.length > 1 && currentPathSegment < currentPath.length - 1) {
            const start = currentPath[currentPathSegment];
            const end = currentPath[currentPathSegment + 1];
            
            // Draw moving request dot
            const x = start.x + (end.x - start.x) * animationProgress;
            const y = start.y + (end.y - start.y) * animationProgress;
            
            ctx.fillStyle = '#0052cc'; // Darker blue
            ctx.strokeStyle = '#007FFF'; // Primary blue
            ctx.beginPath();
            ctx.arc(x, y, 8, 0, 2 * Math.PI);
            ctx.fill();
            ctx.stroke();
        }

        // Draw nodes
        peers.forEach(peer => {
            if (peer === sourceNode) {
                ctx.fillStyle = '#007FFF'; // Primary blue
                ctx.strokeStyle = '#0052cc'; // Darker blue
                ctx.lineWidth = 2;
            } else if (peer === targetNode) {
                ctx.fillStyle = '#007FFF'; // Website link color
                ctx.strokeStyle = '#0052cc'; // Darker shade of link color
                ctx.lineWidth = 2;
            } else {
                ctx.fillStyle = '#007FFF'; // Primary blue
                ctx.strokeStyle = 'rgba(0, 82, 204, 0.2)'; // Dark blue with transparency
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
        
        // Cancel any existing animation
        if (animationFrame) {
            cancelAnimationFrame(animationFrame);
            animationFrame = null;
        }

        const path = findPath(sourceNode, targetNode);
        currentPath = path;
        currentPathSegment = 0;
        animationProgress = 0;

        // Draw the complete path first in a very light blue
        ctx.save();
        ctx.strokeStyle = 'rgba(0, 127, 255, 0.15)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(path[0].x, path[0].y);
        for (let i = 1; i < path.length; i++) {
            ctx.lineTo(path[i].x, path[i].y);
        }
        ctx.stroke();
        ctx.restore();

        // Animation timing setup
        const animationDuration = 500; // Duration per segment in milliseconds
        let startTime = null;

        function animate(currentTime) {
            if (!isPlaying) return;
            
            if (!startTime) {
                startTime = currentTime;
            }

            const elapsed = currentTime - startTime;
            animationProgress = Math.min(elapsed / animationDuration, 1);
            
            if (animationProgress >= 1) {
                currentPathSegment++;
                
                if (currentPathSegment >= currentPath.length - 1) {
                    // Animation complete - wait 1 second then trigger new route
                    setTimeout(() => {
                        if (isPlaying) {
                            startNewRoute();
                        }
                    }, 1000);
                    return;
                }
                
                // Reset for next segment
                startTime = currentTime;
                animationProgress = 0;
            }
            
            draw();
            animationFrame = requestAnimationFrame(animate);
        }

        // Start animation immediately
        animationFrame = requestAnimationFrame(animate);
    }

    let isPlaying = false;
    let routeTimeout;

    function startNewRoute() {
        // Clear any existing timeouts
        if (routeTimeout) {
            clearTimeout(routeTimeout);
            routeTimeout = null;
        }

        // Select source node randomly
        sourceNode = peers[Math.floor(Math.random() * peers.length)];
        
        // Find a target that's at least 1/3 of the way around the ring
        const minDistance = Math.floor(numPeers / 3);
        const sourceIndex = sourceNode.index;
        
        do {
            const targetIndex = Math.floor(Math.random() * peers.length);
            const distance = Math.min(
                Math.abs(targetIndex - sourceIndex),
                numPeers - Math.abs(targetIndex - sourceIndex)
            );
            if (distance >= minDistance) {
                targetNode = peers[targetIndex];
                break;
            }
        } while (true);
        
        currentPath = [];
        animatePath();
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

// Start visualization when DOM is ready
document.addEventListener('DOMContentLoaded', initVisualization);
