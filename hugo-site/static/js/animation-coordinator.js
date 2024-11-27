// Global animation coordinator
const AnimationCoordinator = {
    activeAnimation: null,
    
    setActive(name) {
        if (this.activeAnimation && this.activeAnimation !== name) {
            // Pause other animations
            const event = new CustomEvent('pause-other-animations', {
                detail: { except: name }
            });
            document.dispatchEvent(event);
        }
        this.activeAnimation = name;
    }
};
