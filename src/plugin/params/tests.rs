use truce::params::{ParamFlags, ParamRange};
use truce::prelude::Params;

use super::{PluginParameter, PluginParams};

#[test]
fn all_host_parameters_are_normalized_and_automatable() {
    let infos = PluginParams::param_infos_static();

    assert_eq!(infos.len(), PluginParameter::ALL.len());
    for (parameter, info) in PluginParameter::ALL.into_iter().zip(infos) {
        assert_eq!(info.id, parameter.id());
        assert_eq!(info.name, parameter.display_name());
        assert!(matches!(info.range, ParamRange::Linear { .. }));
        assert_eq!(info.range.min().to_bits(), 0.0_f64.to_bits());
        assert_eq!(info.range.max().to_bits(), 1.0_f64.to_bits());
        assert!(info.flags.contains(ParamFlags::AUTOMATABLE));
        assert!(!info.flags.contains(ParamFlags::HIDDEN));
        assert!(!info.flags.contains(ParamFlags::READONLY));
    }
}
