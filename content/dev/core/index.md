---

title: "Core Development"  
date: 2024-06-11T00:00:00Z  
draft: false  

---

## Introduction

Welcome to the Core Development page for Freenet. This section is dedicated to the development of the Freenet platform itself, ensuring its robustness, scalability, and efficiency. Whether you're an experienced developer or a newcomer, you'll find essential resources and updates here.

## Developer Meetings

{{< latest-news tag="dev-meeting" >}}

## Recently Merged Pull Requests

<div id="merged-pull-requests">
  <p>Loading...</p>
</div>

<script>
  async function fetchMergedPullRequests() {
    const response = await fetch('https://api.github.com/repos/freenet/freenet-core/pulls?state=closed&per_page=5');
    const pullRequests = await response.json();
    const mergedPullRequests = pullRequests.filter(pr => pr.merged_at);

    const container = document.getElementById('merged-pull-requests');
    container.innerHTML = '';

    if (mergedPullRequests.length === 0) {
      container.innerHTML = '<p>No recently merged pull requests found.</p>';
      return;
    }

    const list = document.createElement('ul');
    mergedPullRequests.forEach(pr => {
      const listItem = document.createElement('li');
      listItem.innerHTML = `<a href="${pr.html_url}" target="_blank">${pr.title}</a> by ${pr.user.login}`;
      list.appendChild(listItem);
    });

    container.appendChild(list);
  }

  fetchMergedPullRequests();
</script>
