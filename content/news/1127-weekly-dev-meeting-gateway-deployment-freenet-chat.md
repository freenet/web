+++
title = "Weekly Dev Meeting - Gateway deployment, freenet-chat"
date = 2024-06-10
tags = ["front-page"]
+++

**Freenet Chat Development:**

- Ian has been working on the Freenet chat system and shared a specification document. He decided to focus on a
  web-based interface rather than a command-line interface due to ease of implementation.
- A significant topic was the method for updating contracts within the network. Ian proposed a replace contract field
  that allows for contract updates signed by the contract owner, similar to HTTP 301 redirects.

**Technical Challenges and Solutions:**

- They discussed the challenge of updating user-facing applications while maintaining contract versions. Ignacio
  emphasized the need for a pattern to update the underlying code without disrupting the user experience.
- The team is focusing on stabilizing the current system and ensuring the robustness of the test suite to support quick
  and reliable releases.

**Configuration and Deployment:**

- Ignacio demonstrated improvements in the node configuration, emphasizing the simplicity and reliability of the current
  setup.
- There was a discussion on packaging and distributing the Freenet software, ensuring it runs smoothly on various
  operating systems without requiring root access.
- They plan to create an optimized image for easy deployment, possibly using Docker, and explore automated deployment
  triggered by CI/CD pipelines.

**Gateway Management:**

- They discussed setting up additional gateways on EC2 instances to ensure network stability, especially when Ian is
  unavailable.
- Long-term plans include a decentralized system for gateway management to avoid single points of failure.

#### Action Items:

- Ian to continue working on the freenet-chat prototype.
- Improve node configuration and deployment processes.
- Set up additional gateways to ensure network reliability.
- Stabilize the system and prepare for a gradual rollout to a small group of testers.
