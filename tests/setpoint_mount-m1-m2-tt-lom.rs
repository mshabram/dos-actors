//! M2 fast tip-tilt controller null test
//!
//! Run the mount, M1 force loop, M2 positioner, M2 piezostack and M2 fast tip-tilt feedback loop controllers with the FEM model
//! and with the set points of all the controllers set to 0
//! The optical model uses the Linear Optical Model from the `gmt-lom` crate,
//! the LOM environment variable must be set to the location of the optical
//! sensitivity matrices data file `optical_sensitivities.rs.bin`.
//! The FEM model repository is read from the `FEM_REPO` environment variable
//! The LOM sensitivity matrices are located in the directory given by the `LOM` environment variable

use dos_actors::{
    clients::{
        arrow_client::Arrow,
        fsm::*,
        m1::*,
        mount::{Mount, MountEncoders, MountSetPoint, MountTorques},
    },
    prelude::*,
};
use fem::{
    dos::{DiscreteModalSolver, ExponentialMatrix},
    fem_io::*,
    FEM,
};
use lom::{OpticalMetrics, LOM};
use std::time::Instant;
mod config;
use config::Config;

#[tokio::test]
async fn setpoint_mount_m1_m2_tt_lom() -> anyhow::Result<()> {
    let config = Config::load()?;
    println!("{:?}", config);

    let sim_sampling_frequency = config.sampling;
    let sim_duration = 4_usize;
    let n_step = sim_sampling_frequency * sim_duration;

    let state_space = {
        let fem = FEM::from_env()?.static_from_env()?;
        let n_io = (fem.n_inputs(), fem.n_outputs());
        DiscreteModalSolver::<ExponentialMatrix>::from_fem(fem)
            .sampling(sim_sampling_frequency as f64)
            .proportional_damping(config.damping)
            .ins::<OSSElDriveTorque>()
            .ins::<OSSAzDriveTorque>()
            .ins::<OSSRotDriveTorque>()
            .ins::<OSSHarpointDeltaF>()
            .ins::<M1ActuatorsSegment1>()
            .ins::<M1ActuatorsSegment2>()
            .ins::<M1ActuatorsSegment3>()
            .ins::<M1ActuatorsSegment4>()
            .ins::<M1ActuatorsSegment5>()
            .ins::<M1ActuatorsSegment6>()
            .ins::<M1ActuatorsSegment7>()
            .ins::<MCM2SmHexF>()
            .ins::<MCM2PZTF>()
            .outs::<OSSAzEncoderAngle>()
            .outs::<OSSElEncoderAngle>()
            .outs::<OSSRotEncoderAngle>()
            .outs::<OSSHardpointD>()
            .outs::<MCM2SmHexD>()
            .outs::<MCM2PZTD>()
            .outs::<OSSM1Lcl>()
            .outs::<MCM2Lcl6D>()
            .use_static_gain_compensation(n_io)
            .build()?
    };

    // FEM
    let mut fem: Actor<_> = state_space.into();
    // MOUNT
    let mut mount: Actor<_> = Mount::at_zenith_angle(30)?.into();

    const M1_RATE: usize = 10;
    assert_eq!(sim_sampling_frequency / M1_RATE, 100);

    // HARDPOINTS
    let mut m1_hardpoints: Actor<_> = m1_ctrl::hp_dynamics::Controller::new().into();
    // LOADCELLS
    let mut m1_hp_loadcells: Actor<_, 1, M1_RATE> =
        m1_ctrl::hp_load_cells::Controller::new().into();
    // M1 SEGMENTS ACTUATORS
    let mut m1_segment1: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment1::Controller::new().into();
    let mut m1_segment2: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment2::Controller::new().into();
    let mut m1_segment3: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment3::Controller::new().into();
    let mut m1_segment4: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment4::Controller::new().into();
    let mut m1_segment5: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment5::Controller::new().into();
    let mut m1_segment6: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment6::Controller::new().into();
    let mut m1_segment7: Actor<_, M1_RATE, 1> =
        m1_ctrl::actuators::segment7::Controller::new().into();

    let logging = Arrow::builder(n_step)
        .filename("setpoint-mount-m1-m2-tt-lom.parquet")
        .no_save()
        .build()
        .into_arcx();
    let mut sink = Terminator::<_>::new(logging.clone());

    let mut mount_set_point: Initiator<_> = (Signals::new(3, n_step), "Mount_setpoint").into();
    mount_set_point
        .add_output()
        .build::<MountSetPoint>()
        .into_input(&mut mount);
    mount
        .add_output()
        .build::<MountTorques>()
        .into_input(&mut fem);

    let mut m1rbm_set_point: Initiator<_> = (Signals::new(42, n_step), "M1_RBM").into();
    m1rbm_set_point
        .add_output()
        .build::<M1RBMcmd>()
        .into_input(&mut m1_hardpoints);
    m1_hardpoints
        .add_output()
        .multiplex(2)
        .build::<OSSHarpointDeltaF>()
        .into_input(&mut fem)
        .into_input(&mut m1_hp_loadcells);

    m1_hp_loadcells
        .add_output()
        .build::<S1HPLC>()
        .into_input(&mut m1_segment1);
    m1_hp_loadcells
        .add_output()
        .build::<S2HPLC>()
        .into_input(&mut m1_segment2);
    m1_hp_loadcells
        .add_output()
        .build::<S3HPLC>()
        .into_input(&mut m1_segment3);
    m1_hp_loadcells
        .add_output()
        .build::<S4HPLC>()
        .into_input(&mut m1_segment4);
    m1_hp_loadcells
        .add_output()
        .build::<S5HPLC>()
        .into_input(&mut m1_segment5);
    m1_hp_loadcells
        .add_output()
        .build::<S6HPLC>()
        .into_input(&mut m1_segment6);
    m1_hp_loadcells
        .add_output()
        .build::<S7HPLC>()
        .into_input(&mut m1_segment7);

    m1_segment1
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment1>()
        .into_input(&mut fem);
    m1_segment2
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment2>()
        .into_input(&mut fem);
    m1_segment3
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment3>()
        .into_input(&mut fem);
    m1_segment4
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment4>()
        .into_input(&mut fem);
    m1_segment5
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment5>()
        .into_input(&mut fem);
    m1_segment6
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment6>()
        .into_input(&mut fem);
    m1_segment7
        .add_output()
        .bootstrap()
        .build::<M1ActuatorsSegment7>()
        .into_input(&mut fem);

    const FSM_RATE: usize = 5;
    assert_eq!(sim_sampling_frequency / FSM_RATE, 200);

    // M2 POSITIONER COMMAND
    let mut m2_pos_cmd: Initiator<_> = (Signals::new(42, n_step), "M2_pos").into();
    // FSM POSITIONNER
    let mut m2_positionner: Actor<_> = m2_ctrl::positionner::Controller::new().into();
    m2_pos_cmd
        .add_output()
        .build::<M2poscmd>()
        .into_input(&mut m2_positionner);
    m2_positionner
        .add_output()
        .build::<MCM2SmHexF>()
        .into_input(&mut fem);
    // FSM PIEZOSTACK
    let mut m2_piezostack: Actor<_> = m2_ctrl::piezostack::Controller::new().into();
    m2_piezostack
        .add_output()
        .build::<MCM2PZTF>()
        .into_input(&mut fem);
    // FSM TIP-TILT CONTROL
    let mut tiptilt_set_point: Initiator<_, FSM_RATE> = (
        (0..7).fold(Signals::new(14, n_step), |s, i| {
            (0..2).fold(s, |ss, j| {
                ss.output_signal(
                    i * 2 + j,
                    Signal::Constant((-1f64).powi((i + j) as i32) * 1e-6),
                )
            })
        }),
        "TipTilt_setpoint",
    )
        .into();
    let mut m2_tiptilt: Actor<_, FSM_RATE, 1> = m2_ctrl::tiptilt::Controller::new().into();
    tiptilt_set_point
        .add_output()
        .build::<TTSP>()
        .into_input(&mut m2_tiptilt);
    m2_tiptilt
        .add_output()
        .bootstrap()
        .build::<PZTcmd>()
        .into_input(&mut m2_piezostack);
    // LINEAR OPTICAL MODEL
    let feedback = Logging::default().into_arcx();
    let mut feedback_sink = Terminator::<_, FSM_RATE>::new(feedback.clone());
    let mut lom: Actor<_, 1, FSM_RATE> = LOM::builder().build()?.into();
    lom.add_output()
        .multiplex(2)
        .build::<TTFB>()
        .into_input(&mut m2_tiptilt)
        .into_input(&mut feedback_sink);

    fem.add_output()
        .bootstrap()
        .build::<MountEncoders>()
        .into_input(&mut mount);
    fem.add_output()
        .bootstrap()
        .build::<OSSHardpointD>()
        .into_input(&mut m1_hp_loadcells);
    fem.add_output()
        .multiplex(2)
        .build::<OSSM1Lcl>()
        .into_input(&mut lom)
        .log(&mut sink)
        .await;
    fem.add_output()
        .multiplex(2)
        .build::<MCM2Lcl6D>()
        .into_input(&mut lom)
        .log(&mut sink)
        .await;
    fem.add_output()
        .bootstrap()
        .build::<MCM2SmHexD>()
        .into_input(&mut m2_positionner);
    fem.add_output()
        .bootstrap()
        .build::<MCM2PZTD>()
        .into_input(&mut m2_piezostack);

    Model::new(vec![
        Box::new(mount_set_point),
        Box::new(mount),
        Box::new(m1rbm_set_point),
        Box::new(m1_hardpoints),
        Box::new(m1_hp_loadcells),
        Box::new(m1_segment1),
        Box::new(m1_segment2),
        Box::new(m1_segment3),
        Box::new(m1_segment4),
        Box::new(m1_segment5),
        Box::new(m1_segment6),
        Box::new(m1_segment7),
        Box::new(m2_pos_cmd),
        Box::new(m2_positionner),
        Box::new(m2_piezostack),
        Box::new(tiptilt_set_point),
        Box::new(m2_tiptilt),
        Box::new(lom),
        Box::new(fem),
        Box::new(sink),
        Box::new(feedback_sink),
    ])
    .name("mount-m1-m2-tt-lom")
    .flowchart()
    .check()?
    .run()
    .wait()
    .await?;

    let lom = LOM::builder()
        .rigid_body_motions_record((*logging.lock().await).record()?)?
        .build()?;
    let segment_tiptilt = lom.segment_tiptilt();
    let stt: Vec<_> = segment_tiptilt
        .items()
        .last()
        .unwrap()
        .into_iter()
        .map(|x| x * 1e6)
        .collect();
    println!("STT (last).: {:.3?}", stt);

    println!(
        "STT feedback (last): {:.3?}",
        (*feedback.lock().await)
            .chunks()
            .last()
            .unwrap()
            .into_iter()
            .map(|x| x * 1e6)
            .collect::<Vec<f64>>()
    );

    let stt_residuals = stt
        .chunks(2)
        .enumerate()
        .map(|(i, x)| {
            x.iter()
                .enumerate()
                .map(|(j, x)| x - (-1f64).powi((i + j) as i32))
                .map(|x| x * x)
                .sum::<f64>()
                / 2f64
        })
        .sum::<f64>()
        / 7f64;
    println!(
        "Segment X pupil TT set points RSS error: {}",
        stt_residuals.sqrt()
    );

    assert!(stt_residuals.sqrt() < 1e-3);

    Ok(())
}
