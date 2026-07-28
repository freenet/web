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

    // No burger handler here on purpose. The mobile menu is driven entirely by
    // the #navbar-toggle checkbox and its <label class="navbar-burger">, with
    // the open and closed states in base.css.
    //
    // There used to be a Bulma-style handler that toggled .is-active and read
    // el.dataset.target. This markup has never set data-target, so the lookup
    // returned null and every burger tap threw a TypeError. The menu still
    // opened, because the label had already flipped the checkbox, which is why
    // it went unnoticed.
});
