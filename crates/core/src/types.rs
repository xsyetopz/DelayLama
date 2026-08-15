pub const MIN_SAMPLE_RATE: f64 = 8_000.0;
pub const MAX_SAMPLE_RATE: f64 = 384_000.0;
pub const DEFAULT_SAMPLE_RATE: f64 = 44_100.0;
#[derive(Clone, Copy, Debug, PartialEq)] pub struct VisualState { pub note:i32, pub gate:bool, pub vowel:f32, pub atlas_selector:f32 }
impl Default for VisualState { fn default()->Self{Self{note:-1,gate:false,vowel:0.5,atlas_selector:-1.}} }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] #[repr(u8)] pub enum EventType { #[default] NoteOff, NoteOn, PitchBend, ControlChange, PadPitch, PadVowel }
#[derive(Clone, Copy, Debug, Default)] pub struct Event { pub kind:EventType,pub sample_offset:i32,pub note:i32,pub value:f32,pub controller:i32,pub local_pad:bool }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Parameters { pub vowel:f32,pub port_time:f32,pub delay_mix:f32,pub voice:f32,pub vibrato:f32,pub volume:f32,pub xy_routing:f32 }
impl Default for Parameters { fn default()->Self{Self{vowel:0.5,port_time:0.5,delay_mix:0.8,voice:0.5,vibrato:0.,volume:0.1,xy_routing:0.}} }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub struct VoiceState { pub current_note:i32,pub gate:bool }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct PadState { pub pitch_modulation:f32,pub vowel:f32,pub active:bool }
impl Default for PadState {fn default()->Self{Self{pitch_modulation:0.5,vowel:0.5,active:false}}}
