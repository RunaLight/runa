-- THIS FILE IS AUTO-GENERATED. DO NOT EDIT BY HAND.
-- Source of truth: `#[derive(Scriptable)]` registrations (collected via `inventory`)
-- plus built-in math / KeyCode types. Read by `luau-lsp` for type checking.

export type Vec3 = { x: number, y: number, z: number }
export type Vec4 = { x: number, y: number, z: number, w: number }
export type Vec2 = { x: number, y: number }
export type Quat = { x: number, y: number, z: number, w: number }

export type KeyCode = "KeyA" | "KeyB" | "KeyC" | "KeyD" | "KeyE" | "KeyF" | "KeyG" | "KeyH" | "KeyI" | "KeyJ" | "KeyK" | "KeyL" | "KeyM" | "KeyN" | "KeyO" | "KeyP" | "KeyQ" | "KeyR" | "KeyS" | "KeyT" | "KeyU" | "KeyV" | "KeyW" | "KeyX" | "KeyY" | "KeyZ" | "Digit0" | "Digit1" | "Digit2" | "Digit3" | "Digit4" | "Digit5" | "Digit6" | "Digit7" | "Digit8" | "Digit9" | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "Space" | "Enter" | "Escape" | "Tab" | "Backspace" | "Delete" | "Home" | "End" | "PageUp" | "PageDown" | "Insert" | "ShiftLeft" | "ShiftRight" | "ControlLeft" | "ControlRight" | "AltLeft" | "AltRight" | "Meta" | "SuperLeft" | "SuperRight" | "CapsLock" | "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11" | "F12" | "Backquote" | "Minus" | "Equal" | "BracketLeft" | "BracketRight" | "Backslash" | "Semicolon" | "Quote" | "Comma" | "Period" | "Slash" | "Numpad0" | "Numpad1" | "Numpad2" | "Numpad3" | "Numpad4" | "Numpad5" | "Numpad6" | "Numpad7" | "Numpad8" | "Numpad9" | "NumpadAdd" | "NumpadSubtract" | "NumpadMultiply" | "NumpadDivide" | "NumpadDecimal" | "NumpadEnter"

export type ScriptInput = {
    KeyA: boolean,
    KeyB: boolean,
    KeyC: boolean,
    KeyD: boolean,
    KeyE: boolean,
    KeyF: boolean,
    KeyG: boolean,
    KeyH: boolean,
    KeyI: boolean,
    KeyJ: boolean,
    KeyK: boolean,
    KeyL: boolean,
    KeyM: boolean,
    KeyN: boolean,
    KeyO: boolean,
    KeyP: boolean,
    KeyQ: boolean,
    KeyR: boolean,
    KeyS: boolean,
    KeyT: boolean,
    KeyU: boolean,
    KeyV: boolean,
    KeyW: boolean,
    KeyX: boolean,
    KeyY: boolean,
    KeyZ: boolean,
    Digit0: boolean,
    Digit1: boolean,
    Digit2: boolean,
    Digit3: boolean,
    Digit4: boolean,
    Digit5: boolean,
    Digit6: boolean,
    Digit7: boolean,
    Digit8: boolean,
    Digit9: boolean,
    ArrowUp: boolean,
    ArrowDown: boolean,
    ArrowLeft: boolean,
    ArrowRight: boolean,
    Space: boolean,
    Enter: boolean,
    Escape: boolean,
    Tab: boolean,
    Backspace: boolean,
    Delete: boolean,
    Home: boolean,
    End: boolean,
    PageUp: boolean,
    PageDown: boolean,
    Insert: boolean,
    ShiftLeft: boolean,
    ShiftRight: boolean,
    ControlLeft: boolean,
    ControlRight: boolean,
    AltLeft: boolean,
    AltRight: boolean,
    Meta: boolean,
    SuperLeft: boolean,
    SuperRight: boolean,
    CapsLock: boolean,
    F1: boolean,
    F2: boolean,
    F3: boolean,
    F4: boolean,
    F5: boolean,
    F6: boolean,
    F7: boolean,
    F8: boolean,
    F9: boolean,
    F10: boolean,
    F11: boolean,
    F12: boolean,
    Backquote: boolean,
    Minus: boolean,
    Equal: boolean,
    BracketLeft: boolean,
    BracketRight: boolean,
    Backslash: boolean,
    Semicolon: boolean,
    Quote: boolean,
    Comma: boolean,
    Period: boolean,
    Slash: boolean,
    Numpad0: boolean,
    Numpad1: boolean,
    Numpad2: boolean,
    Numpad3: boolean,
    Numpad4: boolean,
    Numpad5: boolean,
    Numpad6: boolean,
    Numpad7: boolean,
    Numpad8: boolean,
    Numpad9: boolean,
    NumpadAdd: boolean,
    NumpadSubtract: boolean,
    NumpadMultiply: boolean,
    NumpadDivide: boolean,
    NumpadDecimal: boolean,
    NumpadEnter: boolean,
    keys_just_pressed: { [string]: boolean },
    mouse_buttons_pressed: { [string]: boolean },
    mouse_buttons_just_pressed: { [string]: boolean },
    mouse_position: { x: number, y: number },
    mouse_delta: { x: number, y: number },
    mouse_wheel_delta: number,
    is_key_pressed: (self: ScriptInput, key: KeyCode) -> boolean,
    is_key_just_pressed: (self: ScriptInput, key: KeyCode) -> boolean,
    is_mouse_pressed: (self: ScriptInput, button: string) -> boolean,
    is_mouse_just_pressed: (self: ScriptInput, button: string) -> boolean,
    get_mouse_position: (self: ScriptInput) -> { x: number, y: number },
}

export type ScriptEvent = { name: string, x: number, y: number }

export type Transform = {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}
export type Collider2D = {
    offset: Vec2,
    enabled: boolean,
    is_trigger: boolean,
    layer: number,
}
export type SpriteAnimationClip = {
    name: string,
    start_frame: number,
    end_frame: number,
    fps: number,
    looping: boolean,
}
export type SpriteAnimator = {
    sheet: SpriteSheet,
    clips: { [number]: SpriteAnimationClip },
    current_clip: string,
    current_frame: number,
    playing: boolean,
}
export type SpriteSheet = {
    columns: number,
    rows: number,
}
export type MeshRenderer = {
    mesh_path: string,
    visible: boolean,
    cast_shadows: boolean,
    receive_shadows: boolean,
    color: { [number]: number },
}
export type ObjectDefinitionInstance = {
    object_id: string,
}
export type Tile = {
    uv_rect: Rect,
    flip_x: boolean,
    flip_y: boolean,
}
export type TilemapRenderer = {
}
export type Tilemap = {
    width: number,
    height: number,
    layers: { [number]: TilemapLayer },
    selected_tile: number,
    pixels_per_unit: number,
}
export type TilemapLayer = {
    name: string,
    width: number,
    height: number,
    tiles: { [number]: Tile },
    visible: boolean,
    opacity: number,
    self_order: number,
}
export type Rect = {
    x: number,
    y: number,
    width: number,
    height: number,
}
export type ScreenEffects = {
    fade_color: { [number]: number },
    vignette_strength: number,
    vignette_radius: number,
    vignette_softness: number,
    rgb_shift: { [number]: number },
    tint_color: { [number]: number },
    brightness: number,
    contrast: number,
}
export type AudioListener = {
    volume: number,
    active: boolean,
    stereo_separation: number,
}
export type Sorting = {
    order: number,
    y_sort: boolean,
    y_offset: number,
}
export type Camera = {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    orthographic_size: Vec2,
    near: number,
    far: number,
    fov: number,
}
export type CursorInteractable = {
    is_pressed: boolean,
    is_hovered: boolean,
    was_hovered: boolean,
    bounds_size: Vec3,
}
export type Collider3D = {
    offset: Vec3,
    enabled: boolean,
    is_trigger: boolean,
    layer: number,
}
export type PointLight = {
    color: Color,
    intensity: number,
    radius: number,
    falloff: number,
}
export type DirectionalLight = {
    direction: Vec3,
    color: Color,
    intensity: number,
}
export type WorldAtmosphere = {
    ambient_color: Color,
    ambient_intensity: number,
    background_intensity: number,
}
export type SpriteRenderer = {
    texture_path: string,
    pixels_per_unit: number,
    uv_rect: { [number]: number },
    color: { [number]: number },
    replace_color: boolean,
    flip_x: boolean,
    flip_y: boolean,
}
export type ActiveCamera = {
}
export type AudioSource = {
    source_path: string,
    volume: number,
    looped: boolean,
    playing: boolean,
    play_on_awake: boolean,
    play_requested: boolean,
    stop_requested: boolean,
    min_distance: number,
    max_distance: number,
    spatial: boolean,
}
export type PhysicsCollision = {
    size: Vec2,
    enabled: boolean,
}
export type Color = {
    r: number,
    g: number,
    b: number,
    a: number,
}
export type Speed = {
    value: number,
}

export type ScriptContext = {
    entity: number,
    dt: number,
    input: ScriptInput,
    components: { [string]: any },
    events: { ScriptEvent },
    events_in: { ScriptEvent },
    GetComponent: <T>(self: ScriptContext, component: T) -> T,
    HasComponent: <T>(self: ScriptContext, component: T) -> boolean,
    AddComponent: <T>(self: ScriptContext, component: T, value: any) -> (),
    RemoveComponent: <T>(self: ScriptContext, component: T) -> (),
    Spawn: (self: ScriptContext, path: string) -> number,
    Destroy: (self: ScriptContext, entity: number) -> (),
}

local Transform: Transform = {} :: Transform;
local Collider2D: Collider2D = {} :: Collider2D;
local SpriteAnimationClip: SpriteAnimationClip = {} :: SpriteAnimationClip;
local SpriteAnimator: SpriteAnimator = {} :: SpriteAnimator;
local SpriteSheet: SpriteSheet = {} :: SpriteSheet;
local MeshRenderer: MeshRenderer = {} :: MeshRenderer;
local ObjectDefinitionInstance: ObjectDefinitionInstance = {} :: ObjectDefinitionInstance;
local Tile: Tile = {} :: Tile;
local TilemapRenderer: TilemapRenderer = {} :: TilemapRenderer;
local Tilemap: Tilemap = {} :: Tilemap;
local TilemapLayer: TilemapLayer = {} :: TilemapLayer;
local Rect: Rect = {} :: Rect;
local ScreenEffects: ScreenEffects = {} :: ScreenEffects;
local AudioListener: AudioListener = {} :: AudioListener;
local Sorting: Sorting = {} :: Sorting;
local Camera: Camera = {} :: Camera;
local CursorInteractable: CursorInteractable = {} :: CursorInteractable;
local Collider3D: Collider3D = {} :: Collider3D;
local PointLight: PointLight = {} :: PointLight;
local DirectionalLight: DirectionalLight = {} :: DirectionalLight;
local WorldAtmosphere: WorldAtmosphere = {} :: WorldAtmosphere;
local SpriteRenderer: SpriteRenderer = {} :: SpriteRenderer;
local ActiveCamera: ActiveCamera = {} :: ActiveCamera;
local AudioSource: AudioSource = {} :: AudioSource;
local PhysicsCollision: PhysicsCollision = {} :: PhysicsCollision;
local Color: Color = {} :: Color;
local Speed: Speed = {} :: Speed;

return {
    Transform = Transform,
    Collider2D = Collider2D,
    SpriteAnimationClip = SpriteAnimationClip,
    SpriteAnimator = SpriteAnimator,
    SpriteSheet = SpriteSheet,
    MeshRenderer = MeshRenderer,
    ObjectDefinitionInstance = ObjectDefinitionInstance,
    Tile = Tile,
    TilemapRenderer = TilemapRenderer,
    Tilemap = Tilemap,
    TilemapLayer = TilemapLayer,
    Rect = Rect,
    ScreenEffects = ScreenEffects,
    AudioListener = AudioListener,
    Sorting = Sorting,
    Camera = Camera,
    CursorInteractable = CursorInteractable,
    Collider3D = Collider3D,
    PointLight = PointLight,
    DirectionalLight = DirectionalLight,
    WorldAtmosphere = WorldAtmosphere,
    SpriteRenderer = SpriteRenderer,
    ActiveCamera = ActiveCamera,
    AudioSource = AudioSource,
    PhysicsCollision = PhysicsCollision,
    Color = Color,
    Speed = Speed,
    vec2 = function(x: number, y: number): Vec2 return { x = x, y = y } end,
    vec3 = function(x: number, y: number, z: number): Vec3 return { x = x, y = y, z = z } end,
    normalize2 = function(v: Vec2): Vec2 return v end,
    normalize3 = function(v: Vec3): Vec3 return v end,
    length2 = function(v: Vec2): number return 0 end,
    length3 = function(v: Vec3): number return 0 end,
    pi = 3.141592653589793,
    cos = function(x: number): number return 0 end,
    sin = function(x: number): number return 0 end,
    tan = function(x: number): number return 0 end,
    atan2 = function(y: number, x: number): number return 0 end,
    sqrt = function(x: number): number return 0 end,
    abs = function(x: number): number return 0 end,
    floor = function(x: number): number return 0 end,
    ceil = function(x: number): number return 0 end,
    round = function(x: number): number return 0 end,
    sign = function(x: number): number return 0 end,
    pow = function(x: number, y: number): number return 0 end,
    max = function(a: number, b: number): number return 0 end,
    min = function(a: number, b: number): number return 0 end,
    clamp = function(x: number, lo: number, hi: number): number return 0 end,
    rad = function(x: number): number return 0 end,
    deg = function(x: number): number return 0 end,
}
