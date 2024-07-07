---
title: "Donation Successful"
date: 2024-06-24
draft: false
url: "/donate/certified/success"
---

## Thank You for Your Donation!

Your donation to Freenet has been successfully processed. We greatly appreciate your support!

<div id="certificate-info">
  <p>Your donation certificate is being generated. Please wait...</p>
</div>

<div id="certificateSection" style="display: none;">
  <h3>Your Donation Certificate</h3>
  <p>Signed Public Key (base64):</p>
  <textarea id="signedPublicKey" rows="4" cols="50" readonly></textarea>
  <button id="downloadCertificate">Download Certificate</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

{{ $elliptic := resources.Get "https://cdn.jsdelivr.net/npm/elliptic@6.5.4/dist/elliptic.min.js" }}
<script src="{{ $elliptic.RelPermalink }}"></script>
{{ $donationSuccess := resources.Get "js/donation-success.js" | minify }}
<script src="{{ $donationSuccess.RelPermalink }}"></script>

If you have any questions or concerns, please don't hesitate to [contact us](/community/support).

Thank you again for supporting Freenet!
