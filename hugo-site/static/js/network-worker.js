// Network simulation worker

self.onmessage = function(e) {
    const { numPeers, maxPeers } = e.data;
    
    // Calculate network metrics
    const peers = [];
    const links = [];
    const radius = 200;
    const connectionProbability = (distance) => 1 / (distance + 1);

    // Calculate positions for peers
    for (let i = 0; i < numPeers; i++) {
        const angle = (i / numPeers) * 2 * Math.PI;
        peers.push({
            x: 250 + radius * Math.cos(angle),
            y: 250 + radius * Math.sin(angle),
            index: i
        });
    }

    // Generate links
    for (let i = 0; i < numPeers; i++) {
        const nextIndex = (i + 1) % numPeers;
        links.push({ 
            source: peers[i], 
            target: peers[nextIndex] 
        });
        
        for (let j = i + 2; j < numPeers; j++) {
            const distance = Math.min(Math.abs(i - j), numPeers - Math.abs(i - j));
            const prob = connectionProbability(distance);
            if (Math.random() < prob) {
                links.push({ 
                    source: peers[i], 
                    target: peers[j] 
                });
            }
        }
    }

    // Calculate average path length
    let totalLength = 0;
    let pathCount = 0;
    const sampleSize = numPeers > 100 ? 200 : numPeers * 2;
    
    for (let k = 0; k < sampleSize; k++) {
        const i = Math.floor(Math.random() * peers.length);
        const j = Math.floor(Math.random() * peers.length);
        if (i !== j) {
            const path = findShortestPath(peers[i], peers[j], links);
            if (path) {
                totalLength += path.length - 1;
                pathCount++;
            }
        }
    }

    const avgPathLength = totalLength / pathCount;

    self.postMessage({
        peers,
        links,
        avgPathLength,
        numPeers
    });
};

function findShortestPath(start, end, links) {
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
