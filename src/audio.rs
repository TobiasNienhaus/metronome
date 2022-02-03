use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Write};
use std::ops::Add;
use std::process::exit;
use std::sync::atomic::{Ordering as SyncOrdering, AtomicBool, AtomicU16};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, SendError, TryRecvError};
use std::thread::JoinHandle;
use chrono::Timelike;
use cpal::{Data, DefaultStreamConfigError, Devices, DevicesError, Sample, SampleFormat, SupportedOutputConfigs, SupportedStreamConfig, SupportedStreamConfigsError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{debug, error, warn};

use anyhow;

use super::util;

static AUDIO_FLAG: AtomicBool = AtomicBool::new(false);
static AUDIO_VOLUME: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Copy, Clone)]
enum InternalAudioMessage {
    Shutdown,
    External(AudioMessage)
}

#[derive(Debug, Copy, Clone)]
pub enum AudioMessage {
    Play,
    Pause,
    Toggle,
    SetBpm(u16),
    SetVolume(u16),
}

pub struct AudioHandle {
    stream: cpal::Stream,
    thread: Option<std::thread::JoinHandle<()>>,
    sender: Sender<InternalAudioMessage>
}

impl Debug for AudioHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.thread {
            None => f.write_str("AudioHandle(running)"),
            Some(_) => f.write_str("AudioHandle(not running)"),
        }
    }
}

impl AudioHandle {
    fn new() -> AudioHandle {
        let stream = stream_setup_for(sample_next).unwrap();
        stream.play().unwrap();

        let (tx, rx) = channel();

        let thread = std::thread::spawn(move || {
            audio_thread(rx);
        });

        AudioHandle {
            stream,
            thread: Some(thread),
            sender: tx
        }
    }

    fn shutdown(&mut self) {
        debug!("Shutting down audio...");
        if self.thread.is_none() {
            warn!("AudioHandle has no thread to join!");
            return;
        }
        if let Err(e) = (&self).sender.send(InternalAudioMessage::Shutdown) {
            warn!("Could not send shutdown message to audio handler.");
        }
        let thread = self.thread.take().unwrap();
        if thread.join().is_err() {
            error!("Audio handler thread has panicked on join. Could not join!");
        }
    }

    pub fn send(&self, msg: AudioMessage) {
        if let Err(e) = self.sender.send(InternalAudioMessage::External(msg)) {
            warn!("Could not send message to audio handler. It probably shut down for some reason (msg: {:?})", msg);
        }
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn setup() -> AudioHandle {
    AudioHandle::new()
}

fn audio_thread(rx: Receiver<InternalAudioMessage>) {
    let mut running = true;
    let mut paused = false;

    let mut timing = util::bpm_to_ns(55);
    let busy_timing = 50000000;
    let mut sleep_timing = timing - busy_timing;

    if let Err(e) = thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max) {
        error!("Could not set priority! ({:?})", e);
    }
    debug!("Starting audio loop");
    while running {
        if !paused {
            AUDIO_FLAG.store(true, SyncOrdering::SeqCst);
        }

        let start = std::time::Instant::now();
        let sleep_start = start.add(std::time::Duration::from_nanos(busy_timing as u64));
        // TODO warn if the delta gets to big from the requests value
        let (messages, continue_running) = get_message(paused, &rx);

        running = continue_running;

        // TODO somehow consume all messages
        if continue_running {
            let mut volume_to_change_to = None;
            let mut new_paused_state = paused;
            for msg in messages {
                match msg {
                    InternalAudioMessage::External(msg) => {
                        match msg {
                            AudioMessage::Play => new_paused_state = false,
                            AudioMessage::Pause => new_paused_state = true,
                            AudioMessage::Toggle => new_paused_state = !new_paused_state,
                            AudioMessage::SetBpm(bpm) => {
                                debug!("Setting BPM to {}", bpm);
                                timing = util::bpm_to_ns(bpm as u128);
                                sleep_timing = timing - busy_timing;
                            }
                            AudioMessage::SetVolume(vol) => {
                                volume_to_change_to = Some(vol);
                            }
                        }
                    }
                    InternalAudioMessage::Shutdown => running = false
                }
            }
            if let Some(volume) = volume_to_change_to {
                AUDIO_VOLUME.store(volume, SyncOrdering::SeqCst);
            }

            if paused != new_paused_state {
                paused = new_paused_state;
                if paused {
                    AUDIO_FLAG.store(false, SyncOrdering::SeqCst);
                }
            }

            if !paused {
                util::busy_sleep_from(start, busy_timing);
                AUDIO_FLAG.store(false, SyncOrdering::SeqCst);
                // TODO set all values in the smaller phase (usually silent time)
                util::busy_sleep_from(sleep_start, sleep_timing);
            }
        }
    }
}

fn get_message(paused: bool, rx: &Receiver<InternalAudioMessage>) -> (Vec<InternalAudioMessage>, bool) {
    let mut ret = Vec::new();
    let mut should_continue = true;

    if !paused {
        'breakable: loop {
            match rx.try_recv() {
                Ok(msg) => ret.push(msg),
                Err(e) => {
                    if let TryRecvError::Disconnected = e {
                        error!("Audio controller thread channel hung up!");
                        should_continue = false;
                    }
                    break 'breakable;
                }
            }
        }
    } else {
        match rx.recv() {
            Ok(msg) => ret.push(msg),
            Err(_) => {
                error!("Audio controller thread channel hung up!");
                should_continue = false;
            }
        }
    };

    (ret, should_continue)
}

fn sample_next(o: &mut SampleRequestOptions, active: bool, vol: u16) -> f32 {
    o.tick();
    if active {
        o.tone(659.25) * ((vol as f32) / 1000.)
    } else {
        0.
    }
    // combination of several tones
}

#[derive(Debug)]
pub struct SampleRequestOptions {
    pub sample_rate: f32,
    pub sample_clock: f32,
    pub nchannels: usize,
}

impl SampleRequestOptions {
    fn tone(&self, freq: f32) -> f32 {
        match (self.sample_clock * freq * 2.0 * std::f32::consts::PI / self.sample_rate).sin().partial_cmp(&0.0).unwrap() {
            Ordering::Less => -1.,
            Ordering::Equal => 0.,
            Ordering::Greater => 1.
        }
    }
    fn tick(&mut self) {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;
    }
}

pub fn stream_setup_for<F>(on_sample: F) -> Result<cpal::Stream, anyhow::Error>
    where
        F: FnMut(&mut SampleRequestOptions, bool, u16) -> f32 + std::marker::Send + 'static + Copy,
{
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => {
            debug!("F32");
            stream_make::<f32, _>(&device, &config.into(), on_sample)
        },
        cpal::SampleFormat::I16 => {
            debug!("I16");
            stream_make::<i16, _>(&device, &config.into(), on_sample)
        },
        cpal::SampleFormat::U16 => {
            debug!("U16");
            stream_make::<u16, _>(&device, &config.into(), on_sample)
        },
    }
}

pub fn host_device_setup(
) -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig), anyhow::Error> {
    let host = {
        // #[cfg(target_os = "windows")]
        // {
        //     cpal::host_from_id(cpal::HostId::Asio).unwrap()
        // }
        // #[cfg(not(target_os = "windows"))]
        // {
        // }
        cpal::default_host()
    };

    let device = host.output_devices()
        .unwrap()
        .filter(|d| d.name().unwrap().contains("Focusrite"))
        .next().unwrap_or(host.default_output_device()
                                               .ok_or_else(|| anyhow::Error::msg("Default output device is not available"))?);

    println!("Output device : {}", device.name()?);

    let config = device.default_output_config()?;
    println!("Default output config : {:?}", config);

    Ok((host, device, config))
}

pub fn stream_make<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    on_sample: F,
) -> Result<cpal::Stream, anyhow::Error>
    where
        T: cpal::Sample,
        F: FnMut(&mut SampleRequestOptions, bool, u16) -> f32 + std::marker::Send + 'static + Copy,
{
    let sample_rate = config.sample_rate.0 as f32;
    let sample_clock = 0f32;
    let nchannels = config.channels as usize;
    let mut request = SampleRequestOptions {
        sample_rate,
        sample_clock,
        nchannels,
    };

    debug!("Request: {:?}", request);

    let err_fn = |err| error!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            on_window(output, &mut request, on_sample)
        },
        err_fn,
    )?;

    debug!("Built stream");

    Ok(stream)
}

fn on_window<T, F>(output: &mut [T], request: &mut SampleRequestOptions, mut on_sample: F)
    where
        T: cpal::Sample,
        F: FnMut(&mut SampleRequestOptions, bool, u16) -> f32 + std::marker::Send + 'static,
{
    let active = AUDIO_FLAG.load(SyncOrdering::SeqCst);
    let volume = AUDIO_VOLUME.load(SyncOrdering::SeqCst);
    for frame in output.chunks_mut(request.nchannels) {
        let value: T = cpal::Sample::from::<f32>(&on_sample(request, active, volume));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
