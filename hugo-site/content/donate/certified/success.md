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

<script>
document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');
  const clientSecret = urlParams.get('payment_intent_client_secret');

  if (paymentIntent && clientSecret) {
    // Here you would typically make an API call to your backend to generate the certificate
    // For now, we'll just display a message
    document.getElementById('certificate-info').innerHTML = `
      <p>Your donation certificate has been generated.</p>
      <p>Payment Intent ID: ${paymentIntent}</p>
      <p>Please save this information for your records.</p>
    `;
  }
});
</script>

If you have any questions or concerns, please don't hesitate to [contact us](/community/support).

Thank you again for supporting Freenet!
