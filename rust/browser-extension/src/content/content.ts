/**
 * Content script for website integration
 * Bridges between web pages and the extension background worker
 */
import { MessageType } from '../shared/messages';

const GHOSTKEY_EVENT_PREFIX = 'freenet-ghostkey:';

interface PageRequest {
  type: 'authenticate' | 'authenticateContract';
  requestId: string;
  challenge: string;
  purpose: string;
  contractAddress?: string;
}

// Listen for requests from the page via custom DOM events
window.addEventListener(`${GHOSTKEY_EVENT_PREFIX}request`, async (event) => {
  const customEvent = event as CustomEvent<PageRequest>;
  const request = customEvent.detail;

  try {
    const response = await handlePageRequest(request);
    dispatchResponse(request.requestId, response);
  } catch (error) {
    dispatchError(request.requestId, error as Error);
  }
});

async function handlePageRequest(request: PageRequest) {
  // Validate origin
  const origin = window.location.origin;

  // Send to background worker
  const response = await chrome.runtime.sendMessage({
    type: MessageType.PAGE_AUTH_REQUEST,
    origin,
    contractAddress: request.contractAddress,
    challenge: request.challenge,
    purpose: request.purpose,
    requestId: request.requestId,
  });

  if (response.error) {
    throw new Error(response.error);
  }

  return response;
}

function dispatchResponse(requestId: string, data: any) {
  window.dispatchEvent(
    new CustomEvent(`${GHOSTKEY_EVENT_PREFIX}response:${requestId}`, {
      detail: { success: true, data },
    })
  );
}

function dispatchError(requestId: string, error: Error) {
  window.dispatchEvent(
    new CustomEvent(`${GHOSTKEY_EVENT_PREFIX}response:${requestId}`, {
      detail: { success: false, error: error.message },
    })
  );
}

// Inject the API script into the page context
function injectPageScript() {
  const script = document.createElement('script');
  script.textContent = `
    (function() {
      class FreenetGhostkey {
        constructor() {
          this.pendingRequests = new Map();

          window.addEventListener('message', (event) => {
            if (event.data?.type?.startsWith('freenet-ghostkey:response:')) {
              const requestId = event.data.type.split(':')[2];
              const pending = this.pendingRequests.get(requestId);
              if (pending) {
                if (event.data.success) {
                  pending.resolve(event.data.data);
                } else {
                  pending.reject(new Error(event.data.error));
                }
                this.pendingRequests.delete(requestId);
              }
            }
          });
        }

        authenticate(options) {
          return this._sendRequest('authenticate', {
            challenge: options.challenge,
            purpose: options.purpose,
          });
        }

        authenticateContract(options) {
          return this._sendRequest('authenticateContract', {
            contractAddress: options.contractAddress,
            challenge: options.challenge,
            purpose: options.purpose,
          });
        }

        _sendRequest(type, payload) {
          const requestId = crypto.randomUUID();

          return new Promise((resolve, reject) => {
            this.pendingRequests.set(requestId, { resolve, reject });

            window.dispatchEvent(new CustomEvent('freenet-ghostkey:request', {
              detail: { type, requestId, ...payload }
            }));

            // Timeout after 2 minutes
            setTimeout(() => {
              if (this.pendingRequests.has(requestId)) {
                this.pendingRequests.delete(requestId);
                reject(new Error('Request timeout'));
              }
            }, 120000);
          });
        }
      }

      window.freenetGhostkey = new FreenetGhostkey();
      console.log('Freenet Ghost Keys API available at window.freenetGhostkey');
    })();
  `;
  (document.head || document.documentElement).appendChild(script);
  script.remove();
}

// Inject on load
injectPageScript();
