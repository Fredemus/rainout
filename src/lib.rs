use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferSizeInfo {
    MaximumSize(u32),
    ConstantSize(u32),
    UnknownSize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioDeviceInfo {
    pub name: String,

    pub min_channels: u16,
    pub max_channels: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioServerInfo {
    pub name: String,
    pub version: Option<String>,
    pub in_devices: Vec<AudioDeviceInfo>,
    pub out_devices: Vec<AudioDeviceInfo>,
    pub sample_rates: Vec<u32>,
    pub buffer_size: BufferSizeInfo,
    pub active: bool,
}

impl AudioServerInfo {
    pub(crate) fn new(name: String, version: Option<String>) -> Self {
        Self {
            name,
            version,
            in_devices: Vec::new(),
            out_devices: Vec::new(),
            sample_rates: Vec::new(),
            buffer_size: BufferSizeInfo::UnknownSize,
            active: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiDeviceInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiServerInfo {
    pub name: String,
    pub version: Option<String>,
    pub in_devices: Vec<MidiDeviceInfo>,
    pub out_devices: Vec<MidiDeviceInfo>,
    pub active: bool,
}

impl MidiServerInfo {
    pub(crate) fn new(name: String, version: Option<String>) -> Self {
        Self {
            name,
            version,
            in_devices: Vec::new(),
            out_devices: Vec::new(),
            active: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AudioDeviceConfig {
    pub device_name: String,
    pub use_num_channels: Option<u16>,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            device_name: String::new(),
            use_num_channels: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AudioServerConfig {
    pub server_name: String,
    pub use_in_devices: Vec<AudioDeviceConfig>,
    pub use_out_devices: Vec<AudioDeviceConfig>,
    pub use_sample_rate: Option<u32>,
    pub use_buffer_size: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MidiDeviceConfig {
    pub device_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MidiServerConfig {
    pub server_name: String,
    pub use_in_devices: Vec<MidiDeviceConfig>,
    pub use_out_devices: Vec<MidiDeviceConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioDeviceStreamInfo {
    pub name: String,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiDeviceStreamInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamInfo {
    pub server_name: String,
    pub audio_in_devices: Vec<AudioDeviceStreamInfo>,
    pub audio_out_devices: Vec<AudioDeviceStreamInfo>,
    pub midi_in_devices: Vec<MidiDeviceStreamInfo>,
    pub midi_out_devices: Vec<MidiDeviceStreamInfo>,
    pub sample_rate: u32,
    pub audio_buffer_size: BufferSizeInfo,
}

pub trait RtProcessHandler: 'static + Send + Sized {
    /// Initialize/allocate any buffers here. This will only be called once
    /// on creation.
    fn init(&mut self, stream_info: &StreamInfo);

    fn process(&mut self, proc_info: ProcessInfo);
}

pub struct AudioDeviceBuffer {
    pub(crate) device_name: String,
    pub(crate) buffers: Vec<Vec<f32>>,
    pub(crate) frames: usize,
}

impl AudioDeviceBuffer {
    pub fn device_name(&self) -> &String {
        &self.device_name
    }

    pub fn get(&self, channel: usize) -> Option<&[f32]> {
        self.buffers.get(channel).map(|b| b.as_slice())
    }

    pub fn get_mut(&mut self, channel: usize) -> Option<&mut [f32]> {
        self.buffers.get_mut(channel).map(|b| b.as_mut_slice())
    }

    pub fn buffers(&self) -> &[Vec<f32>] {
        self.buffers.as_slice()
    }

    pub fn buffers_mut(&mut self) -> &mut [Vec<f32>] {
        self.buffers.as_mut_slice()
    }

    pub fn channels(&self) -> usize {
        self.buffers.len()
    }

    pub fn frames(&self) -> usize {
        self.frames
    }
}

impl std::ops::Index<usize> for AudioDeviceBuffer {
    type Output = [f32];

    fn index(&self, index: usize) -> &Self::Output {
        self.buffers[index].as_slice()
    }
}
impl std::ops::IndexMut<usize> for AudioDeviceBuffer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.buffers[index].as_mut_slice()
    }
}

impl std::fmt::Debug for AudioDeviceBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioDeviceBuffers")
            .field("device_name", &self.device_name)
            .field("channels", &self.buffers.len())
            .field("frames", &self.frames)
            .finish()
    }
}

#[derive(Debug)]
pub struct ProcessInfo<'a> {
    pub audio_in: &'a [AudioDeviceBuffer],
    pub audio_out: &'a mut [AudioDeviceBuffer],
    pub audio_frames: usize,

    pub sample_rate: u32,
    // TODO: MIDI IO
}

#[derive(Debug, Clone, Copy)]
pub struct EstimatedLatency {
    pub frames: u32,
    pub sample_rate: u32,
}

impl EstimatedLatency {
    pub fn as_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(f64::from(self.frames) / f64::from(self.sample_rate))
    }
}

#[derive(Debug)]
pub enum SpawnRtThreadError {
    AudioServerUnavailable(String),
    AudioDeviceNotFoundInServer(String, String),
    NoAudioDeviceSelected(String),
    PlatformSpecific(Box<dyn std::error::Error + Send + 'static>),
}

impl std::error::Error for SpawnRtThreadError {}

impl std::fmt::Display for SpawnRtThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnRtThreadError::AudioServerUnavailable(server) => {
                write!(
                    f,
                    "Error spawning rt thread: The audio sever is unavailable: {:?}.",
                    server
                )
            }
            SpawnRtThreadError::AudioDeviceNotFoundInServer(device, server) => {
                write!(
                    f,
                    "Error spawning rt thread: The audio device {:?} was not found in the audio server {:?}.",
                    device,
                    server
                )
            }
            SpawnRtThreadError::NoAudioDeviceSelected(server) => {
                write!(
                    f,
                    "Error spawning rt thread: No audio device was selected for server {:?}.",
                    server
                )
            }
            SpawnRtThreadError::PlatformSpecific(e) => {
                write!(f, "Error spawning rt thread: Platform error: {:?}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum StreamError {
    AudioServerDisconnected(String),
    AudioDeviceDisconnected(String),
    PlatformSpecific(Box<dyn std::error::Error + Send + 'static>),
}

impl std::error::Error for StreamError {}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::AudioServerDisconnected(server) => {
                write!(
                    f,
                    "Stream error: The audio sever was disconnected: {:?}.",
                    server
                )
            }
            StreamError::AudioDeviceDisconnected(device) => {
                write!(
                    f,
                    "Stream error: The audio device was disconnected: {:?}.",
                    device
                )
            }
            StreamError::PlatformSpecific(e) => {
                write!(f, "Stream error: Platform error: {:?}", e)
            }
        }
    }
}
