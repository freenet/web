+++
tags = ["dev-meeting"]
title = "Weekly Developer Meeting: Updating State and Performance Optimizations"
date = 2023-11-22
+++

We had a detailed technical discussion focusing on various aspects of our project, Freenet. Here's a
breakdown of the key points:

1. **Contract Update Mechanism**: We tackled the contract update mechanism, crucial for handling the
   state (the associated data) of contracts in our key-value store. This involves understanding how
   updates are initiated by applications, the merging of states, and the process of sending updates
   to subscribers.

2. **Update and Merge Process**: We discussed the specifics of how updates work, particularly
   focusing on the 'put' operation. The conversation clarified how 'puts' are handled differently
   depending on whether the application is already subscribed to the contract. A 'put' doesn't
   necessarily need a 'get' first. It's about merging states if the contract exists and managing
   updates accordingly.

3. **Handling Contract Interactions**: Our conversation covered how to handle interactions with
   contracts, including the nuances of 'put' and 'update' requests. We clarified the terminology
   shift, where a 'put' is used when not subscribed to a contract, and it can contain complete state
   or just a delta. An 'update' is more about when we are subscribed, and it involves spreading
   changes across subscribers.

4. **Subscription Mechanism and Network Propagation**: We went in-depth into the subscription
   mechanism, examining how subscription requests are forwarded and managed across the network. This
   included discussing how updates propagate through the network, both upstream and downstream,
   ensuring all peers have the latest state.

5. **Routing and Simulation in the Network**: We explored network routing and how to simulate
   different network conditions. This involved discussing the forwarding of requests and how to
   effectively simulate packet loss or latency to test the network's resilience and efficiency.

6. **Performance Optimizations**: We recognized the need for performance optimizations, especially
   related to speed. We discussed the potential of moving away from libp2p for certain optimizations
   to enhance the system's performance.

7. **Monitoring and Diagnostics for Simulations**: We planned to implement tools for monitoring and
   diagnostics during simulations. This is to ensure we can track necessary data effectively and
   make informed decisions based on simulation outcomes.

8. **Future Development Focus**: We agreed on the importance of having a correct and functioning
   network before moving into deeper optimizations, setting this as our future development priority.

Overall, our discussion was a deep dive into the technicalities of Freenet, focusing on ensuring our
network operations are efficient, effective, and ready for future enhancements.
