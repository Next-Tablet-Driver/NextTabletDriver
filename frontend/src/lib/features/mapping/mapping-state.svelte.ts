import { getConfig, setConfig, saveConfig, type Config } from '../../infrastructure/tauri/commands';
import { untrack } from 'svelte';

const HISTORY_MAX_SIZE = 50;
const DEBOUNCE_SAVE_MS = 1500;

export class MappingState {
    config: Config | null = $state(null);
    isLoaded = $state(false);
    isDirty = $state(false);
    
    // UI bindable properties
    targetW = $state(1920);
    targetH = $state(1080);
    targetX = $state(0);
    targetY = $state(0);
    
    tabletW = $state(0);
    tabletH = $state(0);
    tabletX = $state(0);
    tabletY = $state(0);
    tabletRotation = $state(0);
    
    edgeSnapping = $state(true);
    forceAspectRatio = $state(true);
    showOsuPlayfield = $state(true);
    driverMode = $state("Absolute");

    private historyStack: string[] = [];
    private historyIndex = -1;
    private savedConfigStr = "";
    private saveTimeout?: ReturnType<typeof setTimeout>;

    async load(): Promise<void> {
        this.isLoaded = false;
        try {
            this.config = await getConfig();
            this.applyToState(this.config);

            // Reconstruct config exactly as `subscribeToChanges` does to guarantee JSON serialization order match
            this.config = {
                ...this.config,
                target_area: { x: this.targetX, y: this.targetY, w: this.targetW, h: this.targetH },
                active_area: { x: this.tabletX, y: this.tabletY, w: this.tabletW, h: this.tabletH, rotation: this.tabletRotation },
                display_snapping: this.edgeSnapping,
                lock_aspect_ratio: this.forceAspectRatio,
                show_osu_playfield: this.showOsuPlayfield,
                mode: this.driverMode
            };

            const initStr = JSON.stringify(this.config);
            this.savedConfigStr = initStr;
            this.historyStack = [initStr];
            this.historyIndex = 0;
            this.isDirty = false;

            
            setTimeout(() => { this.isLoaded = true; }, 50);
        } catch (e) {
            console.error("Failed to load config", e);
        }
    }

    private applyToState(cfg: Config): void {
        this.targetW = cfg.target_area.w;
        this.targetH = cfg.target_area.h;
        this.targetX = cfg.target_area.x;
        this.targetY = cfg.target_area.y;

        this.tabletW = cfg.active_area.w;
        this.tabletH = cfg.active_area.h;
        this.tabletX = cfg.active_area.x;
        this.tabletY = cfg.active_area.y;
        this.tabletRotation = cfg.active_area.rotation;

        this.edgeSnapping = cfg.display_snapping;
        this.forceAspectRatio = cfg.lock_aspect_ratio;
        this.showOsuPlayfield = cfg.show_osu_playfield;
        this.driverMode = cfg.mode;
    }
    
    // Call this inside an $effect in a component
    subscribeToChanges(): void {
        if (!this.isLoaded) return;
        
        // Read all properties to track them
        const { targetW, targetH, targetX, targetY, tabletW, tabletH, tabletX, tabletY, tabletRotation, edgeSnapping, forceAspectRatio, showOsuPlayfield, driverMode } = this;
        
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            untrack(() => {
                const currentConfig = this.config;
                if (currentConfig === null) return;
                const updated: Config = {
                    ...currentConfig,
                    target_area: { x: targetX, y: targetY, w: targetW, h: targetH },
                    active_area: { x: tabletX, y: tabletY, w: tabletW, h: tabletH, rotation: tabletRotation },
                    display_snapping: edgeSnapping,
                    lock_aspect_ratio: forceAspectRatio,
                    show_osu_playfield: showOsuPlayfield,
                    mode: driverMode
                };
                setConfig(updated).catch(console.error);
                
                const updatedStr = JSON.stringify(updated);
                if (updatedStr !== this.historyStack[this.historyIndex]) {
                    this.historyStack = this.historyStack.slice(0, this.historyIndex + 1);
                    this.historyStack.push(updatedStr);
                    this.historyIndex++;
                    if (this.historyStack.length > HISTORY_MAX_SIZE) {
                        this.historyStack.shift();
                        this.historyIndex--;
                    }
                }
                
                this.isDirty = (updatedStr !== this.savedConfigStr);
                this.config = updated;
            });
        }, DEBOUNCE_SAVE_MS);
    }
    
    markDirty(): void {
        if (!this.isLoaded || this.config === null) return;
        
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            untrack(() => {
                const currentConfig = this.config;
                if (currentConfig === null) return;
                const updated: Config = {
                    ...currentConfig,
                    target_area: { x: this.targetX, y: this.targetY, w: this.targetW, h: this.targetH },
                    active_area: { x: this.tabletX, y: this.tabletY, w: this.tabletW, h: this.tabletH, rotation: this.tabletRotation },
                    display_snapping: this.edgeSnapping,
                    lock_aspect_ratio: this.forceAspectRatio,
                    show_osu_playfield: this.showOsuPlayfield,
                    mode: this.driverMode
                };
                setConfig(updated).catch(console.error);
                
                const updatedStr = JSON.stringify(updated);
                if (updatedStr !== this.historyStack[this.historyIndex]) {
                    this.historyStack = this.historyStack.slice(0, this.historyIndex + 1);
                    this.historyStack.push(updatedStr);
                    this.historyIndex++;
                    if (this.historyStack.length > HISTORY_MAX_SIZE) {
                        this.historyStack.shift();
                        this.historyIndex--;
                    }
                }
                
                this.isDirty = (updatedStr !== this.savedConfigStr);
                this.config = updated;
            });
        }, DEBOUNCE_SAVE_MS);
    }
    
    undo(): void {
        if (this.historyIndex > 0) {
            this.historyIndex--;
            this.applyHistoryConfig();
        }
    }
    
    redo(): void {
        if (this.historyIndex < this.historyStack.length - 1) {
            this.historyIndex++;
            this.applyHistoryConfig();
        }
    }
    
    revertToInitial(): void {
        if (this.historyStack.length > 0) {
            this.historyIndex = 0;
            this.applyHistoryConfig();
        }
    }
    
    async save(): Promise<void> {
        if (this.isDirty) {
            await saveConfig().catch(console.error);
            this.savedConfigStr = this.historyStack[this.historyIndex];
            this.isDirty = false;
        }
    }
    
    private applyHistoryConfig(): void {
        const historicalConfig = JSON.parse(this.historyStack[this.historyIndex]) as Config;
        this.config = historicalConfig;
        
        this.isLoaded = false;
        this.applyToState(historicalConfig);
        
        setConfig(historicalConfig).catch(console.error);
        this.isDirty = (this.historyStack[this.historyIndex] !== this.savedConfigStr);
        
        setTimeout(() => { this.isLoaded = true; }, 50);
    }
}
