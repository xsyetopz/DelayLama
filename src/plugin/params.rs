//! Host-visible automation parameters and the asset-editor bridge.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crossbeam_queue::ArrayQueue;
use truce::params::{
    FloatParamReadF32, FloatParamReadF64, ParamFlags, ParamInfo, ParamRange, ParamUnit,
    ParamValueKind, SmoothingStyle,
};
use truce::prelude::{FloatParam, Params};

use crate::protocol::GestureResult;

use super::parameter::PluginParameter;

#[derive(Debug)]
pub(super) struct EditorBridge {
    commands: ArrayQueue<GestureResult>,
    animation_frame: AtomicUsize,
}

impl EditorBridge {
    pub(super) fn animation_frame(&self) -> usize {
        self.animation_frame.load(Ordering::Relaxed)
    }

    pub(super) fn publish_animation_frame(&self, frame: usize) {
        self.animation_frame.store(frame, Ordering::Relaxed);
    }

    pub(super) fn pop(&self) -> Option<GestureResult> {
        self.commands.pop()
    }

    pub(super) fn push(&self, command: GestureResult) {
        if let Err(command) = self.commands.push(command) {
            let _evicted_command = self.commands.pop();
            let _retry = self.commands.push(command);
        }
    }
}

impl Default for EditorBridge {
    fn default() -> Self {
        Self {
            commands: ArrayQueue::new(64),
            animation_frame: AtomicUsize::new(5),
        }
    }
}

/// Parameters exposed to a plugin host.
pub struct PluginParams {
    automation: [FloatParam; 4],
    pub(super) editor: Arc<EditorBridge>,
}

impl PluginParams {
    /// Creates the four parameters exposed to a host.
    pub fn new() -> Self {
        Self {
            automation: PluginParameter::ALL
                .map(|parameter| FloatParam::new(parameter_info(parameter), SmoothingStyle::None)),
            editor: Arc::new(EditorBridge::default()),
        }
    }

    fn parameter(&self, id: u32) -> Option<&FloatParam> {
        let parameter = PluginParameter::from_id(id)?;
        self.automation.get(parameter.index())
    }

    pub(super) fn value(&self, parameter: PluginParameter) -> f32 {
        self.automation
            .get(parameter.index())
            .map(FloatParamReadF32::value)
            .unwrap_or_default()
    }

    pub(super) fn info(parameter: PluginParameter) -> ParamInfo {
        parameter_info(parameter)
    }
}

impl Default for PluginParams {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PluginParams {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PluginParams")
            .field(
                "parameters",
                &PluginParameter::ALL.map(PluginParameter::field_name),
            )
            .finish()
    }
}

impl truce::params::__private::Sealed for PluginParams {}

impl Params for PluginParams {
    fn param_infos(&self) -> Vec<ParamInfo> {
        Self::param_infos_static()
    }

    fn param_infos_static() -> Vec<ParamInfo> {
        PluginParameter::ALL
            .into_iter()
            .map(parameter_info)
            .collect()
    }

    fn count(&self) -> usize {
        PluginParameter::ALL.len()
    }

    fn get_normalized(&self, id: u32) -> Option<f64> {
        self.parameter(id).map(|parameter| {
            parameter
                .info
                .range
                .normalize(FloatParamReadF64::value(parameter))
        })
    }

    fn set_normalized(&self, id: u32, value: f64) {
        if let Some(parameter) = self.parameter(id) {
            parameter.set_value(parameter.info.range.denormalize(value));
        }
    }

    fn get_plain(&self, id: u32) -> Option<f64> {
        self.parameter(id).map(FloatParamReadF64::value)
    }

    fn set_plain(&self, id: u32, value: f64) {
        if let Some(parameter) = self.parameter(id) {
            parameter.set_value(value);
        }
    }

    fn format_value(&self, id: u32, value: f64) -> Option<String> {
        self.parameter(id)
            .map(|parameter| truce::params::format_param_value(&parameter.info, value))
    }

    fn parse_value(&self, id: u32, text: &str) -> Option<f64> {
        self.parameter(id).and_then(|_| text.parse().ok())
    }

    fn snap_smoothers(&self) {
        for parameter in &self.automation {
            parameter.smoother.snap(FloatParamReadF64::value(parameter));
        }
    }

    fn set_sample_rate(&self, sample_rate: f64) {
        for parameter in &self.automation {
            parameter.smoother.set_sample_rate(sample_rate);
        }
    }

    fn collect_values(&self) -> (Vec<u32>, Vec<f64>) {
        let ids = PluginParameter::ALL
            .into_iter()
            .map(PluginParameter::id)
            .collect();
        let values = self
            .automation
            .iter()
            .map(FloatParamReadF64::value)
            .collect();
        (ids, values)
    }

    fn restore_values(&self, values: &[(u32, f64)]) {
        for (id, value) in values {
            self.set_plain(*id, *value);
        }
    }
}

fn parameter_info(parameter: PluginParameter) -> ParamInfo {
    let name = parameter.display_name();
    ParamInfo {
        id: parameter.id(),
        name,
        short_name: name,
        group: "",
        range: ParamRange::Linear { min: 0.0, max: 1.0 },
        default_plain: parameter.default(),
        flags: ParamFlags::AUTOMATABLE.union(ParamFlags::CHUNKED),
        unit: ParamUnit::None,
        kind: ParamValueKind::Float,
        midi_map: None,
        midi_channel: None,
    }
}
