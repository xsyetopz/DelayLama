use delaylama_core::*;
#[test] fn sanitises_parameters(){let mut e=SynthEngine::default();e.set_parameters(Parameters{vowel:f32::NAN,..Parameters::default()});assert_eq!(e.parameters().vowel,0.5);}
#[test] fn renders_voice_and_delay(){let mut e=SynthEngine::default();e.prepare(44100.,64,2);let ev=[Event{kind:EventType::NoteOn,note:69,value:1.,..Event::default()}];let mut l=[0.;64];let mut r=[0.;64];e.process(&mut[&mut l,&mut r],64,&ev);assert!(l.iter().any(|x|x.abs()>0.));assert_eq!(e.voice_state().current_note,69);}
#[test] fn controls_pad(){let mut e=SynthEngine::default();e.process(&mut[],1,&[Event{kind:EventType::PadVowel,value:0.8,..Event::default()}]);assert!(e.pad_state().active);assert_eq!(e.pad_state().vowel,0.8);}
