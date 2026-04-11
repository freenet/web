document.addEventListener('DOMContentLoaded', () => {
    // When running inside Freenet's sandboxed iframe, regular link clicks are blocked
    // because the sandbox lacks allow-top-navigation. Intercept clicks on same-origin
    // links and navigate within the iframe instead.
    if (window.location.search.includes('__sandbox=1')) {
        document.addEventListener('click', (e) => {
            const link = e.target.closest('a[href]');
            if (!link) return;
            const href = link.getAttribute('href');
            // Only intercept internal links (absolute paths starting with /)
            if (href && href.startsWith('/') && !href.startsWith('//')) {
                e.preventDefault();
                window.location.href = href;
            }
        });
    }

    // Get all "navbar-burger" elements
    const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);
  
    // Check if there are any navbar burgers
    if ($navbarBurgers.length > 0) {
  
      // Add a click event on each of them
      $navbarBurgers.forEach(el => {
        el.addEventListener('click', () => {
  
          // Get the target from the "data-target" attribute
          const target = el.dataset.target;
          const $target = document.getElementById(target);
  
          // Toggle the "is-active" class on both the "navbar-burger" and the "navbar-menu"
          el.classList.toggle('is-active');
          $target.classList.toggle('is-active');
  
        });
      });
    }
  });
  