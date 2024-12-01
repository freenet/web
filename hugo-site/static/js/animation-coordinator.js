/**
 * Global animation coordinator for managing multiple animations on a page.
 * Ensures only one animation is active at a time and handles state transitions.
 */
const AnimationCoordinator = {
    /** @type {string|null} Current active animation name */
    activeAnimation: null,
    
    /** @type {Map<string, Object>} State storage for animations */
    states: new Map(),

    /**
     * Sets an animation as active and pauses others
     * @param {string} name - Unique identifier for the animation
     * @param {Object} [initialState={}] - Optional initial state for the animation
     */
    setActive(name, initialState = {}) {
        if (this.activeAnimation && this.activeAnimation !== name) {
            // Pause other animations
            const event = new CustomEvent('pause-other-animations', {
                detail: { except: name }
            });
            document.dispatchEvent(event);
        }
        this.activeAnimation = name;
        
        // Initialize or update state
        if (!this.states.has(name)) {
            this.states.set(name, initialState);
        }
    },

    /**
     * Gets the state for a specific animation
     * @param {string} name - Animation identifier
     * @returns {Object|null} Animation state or null if not found
     */
    getState(name) {
        return this.states.get(name) || null;
    },

    /**
     * Updates state for an animation
     * @param {string} name - Animation identifier
     * @param {Object} newState - New state to merge with existing
     */
    updateState(name, newState) {
        const currentState = this.states.get(name) || {};
        this.states.set(name, { ...currentState, ...newState });
    },

    /**
     * Checks if an animation is currently active
     * @param {string} name - Animation identifier
     * @returns {boolean} True if this is the active animation
     */
    isActive(name) {
        return this.activeAnimation === name;
    },

    /**
     * Resets state for an animation
     * @param {string} name - Animation identifier
     */
    resetState(name) {
        this.states.delete(name);
    },

    /**
     * Pauses all animations
     */
    pauseAll() {
        const event = new CustomEvent('pause-other-animations', {
            detail: { except: null }
        });
        document.dispatchEvent(event);
        this.activeAnimation = null;
    }
};
