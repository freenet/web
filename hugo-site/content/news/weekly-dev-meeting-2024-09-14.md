+++
tags = ["dev-meeting", "front-page"]
title = "Weekly Dev Meeting - Finalizing Peer Connections and Preparing for Alpha Release" 
date = 2024-09-14
+++

Just a brief update this week as we work towards the alpha release.

### Gateway Connection Handling

After a few more fixes, connections with gateways are now handled smoothly. Although transport may
still fail sporadically, we now appropriately retry connections, which resolves many of the previous
issues.

### Regular Peer Connections

Regular peers can now connect with each other! While there is still some weirdness that we are
investigating, the connections are finally working as expected.

### Upcoming Plans

These changes are looking good for merging into the main branch soon. Once merged, we'll start
cleaning things up in preparation for an upcoming release.
