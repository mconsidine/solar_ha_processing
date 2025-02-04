use crate::subs::runnable::RunnableSubcommand;

use sciimg::path;
use solhat::{drizzle, enums::Target, processing};
use std::process;

#[derive(clap::Args)]
#[clap(author, version, about = "Process a full observation", long_about = None)]
pub struct Process {
    #[clap(long, short, help = "Input ser files", multiple_values(true))]
    input_files: Vec<String>,

    #[clap(long, short, help = "Output image")]
    output: String,

    #[clap(long, short, help = "Flat frame file")]
    flat: Option<String>,

    #[clap(long, short, help = "Dark frame file")]
    dark: Option<String>,

    #[clap(long, short = 'D', help = "Dark Flat frame file")]
    darkflat: Option<String>,

    #[clap(long, short, help = "Bias frame file")]
    bias: Option<String>,

    #[clap(long, short, help = "Crop width")]
    width: Option<usize>,

    #[clap(long, short = 'H', help = "Crop height")]
    height: Option<usize>,

    #[clap(long, short, help = "Observer latitude", allow_hyphen_values(true))]
    latitude: f32,

    #[clap(
        long,
        short = 'L',
        help = "Observer longitude",
        allow_hyphen_values(true)
    )]
    longitude: f32,

    #[clap(long, short, help = "Object detection threshold")]
    threshold: Option<f32>,

    #[clap(long, short, help = "Image mask")]
    mask: Option<String>,

    #[clap(long, short, help = "Quality limit (top % frames)")]
    quality: Option<u8>,

    #[clap(long, short = 's', help = "Minimum sigma value")]
    minsigma: Option<f32>,

    #[clap(long, short = 'S', help = "Maximum sigma value")]
    maxsigma: Option<f32>,

    #[clap(
        long,
        short = 'I',
        help = "Force an initial rotation value",
        allow_hyphen_values(true)
    )]
    rotation: Option<f64>,

    #[clap(
        long,
        short = 'P',
        help = "Scale maximum value to percentage max possible (0-100)"
    )]
    percentofmax: Option<f32>,

    #[clap(long, short, help = "Number of frames (default=all)")]
    number_of_frames: Option<usize>,

    #[clap(long, short = 'T', help = "Target (Moon, Sun)")]
    target: Option<String>,

    #[clap(long, help = "Disable parallactic rotation")]
    norot: bool,

    #[clap(long, short = 'u', help = "Drizze upscale (1.5, 2.0, 3.0")]
    drizzle: Option<String>,

    #[clap(long, short = 'r', help = "Process report path")]
    report: Option<String>,
}

impl RunnableSubcommand for Process {
    fn run(&self) {
        if !path::parent_exists_and_writable(&self.output) {
            eprintln!(
                "Error: Output parent directory does not exist or is unwritable: {}",
                path::get_parent(&self.output)
            );
            process::exit(2);
        }

        let target = match &self.target {
            Some(t) => match Target::from(t) {
                Some(t) => t,
                None => {
                    eprintln!("Error: Unrecognized target value: {}", t);
                    process::exit(1);
                }
            },
            None => Target::Sun,
        };

        let obj_detect_threshold = self.threshold.unwrap_or(40.0);
        let crop_width = self.width.unwrap_or(0);
        let crop_height = self.height.unwrap_or(0);

        if crop_width == 0 && crop_height > 0 || crop_width > 0 && crop_height == 0 {
            eprintln!("Error: Both width and height need to be specified if any are");
            process::exit(1);
        }

        let flat_frame = match &self.flat {
            Some(f) => {
                if !path::file_exists(f) {
                    eprintln!("Error: Flat file not found: {}", f);
                }
                f.clone()
            }
            None => String::from(""),
        };

        let dark_frame = match &self.dark {
            Some(f) => {
                if !path::file_exists(f) {
                    eprintln!("Error: Dark file not found: {}", f);
                }
                f.clone()
            }
            None => String::from(""),
        };

        let dark_flat_frame = match &self.darkflat {
            Some(f) => {
                if !path::file_exists(f) {
                    eprintln!("Error: Dark flat file not found: {}", f);
                }
                f.clone()
            }
            None => String::from(""),
        };

        let bias_frame = match &self.bias {
            Some(f) => {
                if !path::file_exists(f) {
                    eprintln!("Error: Bias file not found: {}", f);
                }
                f.clone()
            }
            None => String::from(""),
        };

        let mask_file = match &self.mask {
            Some(f) => {
                if !path::file_exists(f) {
                    eprintln!("Error: Mask file not found: {}", f);
                }
                f.clone()
            }
            None => String::from(""),
        };

        let red_scalar = 1.0;
        let green_scalar = 1.0;
        let blue_scalar = 1.0;
        let max_sigma = self.maxsigma.unwrap_or(1000000.0);
        let min_sigma = self.minsigma.unwrap_or(0.0);

        let initial_rotation = self.rotation;
        let obs_latitude = self.latitude;
        let obs_longitude = self.longitude;

        let limit_top_pct = match self.quality {
            Some(p) => {
                if p > 100 {
                    panic!("Error: Quality limit percentage cannot exceed 100%");
                } else {
                    p
                }
            }
            None => 100,
        };

        let number_of_frames = self.number_of_frames.unwrap_or(10000000);

        let pct_of_max = match self.percentofmax {
            Some(p) => {
                if p <= 0.0 {
                    panic!("Error: Percentage cannot be zero or below");
                } else if p > 100.0 {
                    panic!("Error: Percentage cannot exceed 100%");
                } else {
                    p
                }
            }
            None => 100.0,
        };

        let drizzle_scale = match &self.drizzle {
            Some(s) => match s.as_str() {
                "1.0" => drizzle::Scale::Scale1_0,
                "1.5" => drizzle::Scale::Scale1_5,
                "2.0" => drizzle::Scale::Scale2_0,
                "3.0" => drizzle::Scale::Scale3_0,
                _ => {
                    eprintln!(
                        "Invalid drizze scale: {}. Valid options: 1.0, 1.5, 2.0, 3.0",
                        s
                    );
                    process::exit(1);
                }
            },
            None => drizzle::Scale::Scale1_0,
        };

        let enable_rotation = !self.norot;

        let input_files: Vec<&str> = self.input_files.iter().map(|s| s.as_str()).collect();

        let mut ha_processing = processing::HaProcessing::init_new(
            &input_files,
            &flat_frame,
            &dark_frame,
            &dark_flat_frame,
            &bias_frame,
            &mask_file,
            crop_width,
            crop_height,
            obj_detect_threshold,
            red_scalar,
            green_scalar,
            blue_scalar,
            obs_latitude,
            obs_longitude,
            min_sigma,
            max_sigma,
            pct_of_max,
            number_of_frames,
            target,
            drizzle_scale,
        )
        .expect("Failed to create processing context");
        ha_processing.process_ser_files(
            &input_files,
            limit_top_pct,
            enable_rotation,
            initial_rotation,
            |_ps, _frame_no| {},
            |_ps| {},
        );
        ha_processing
            .finalize(&self.output)
            .expect("Failed to finalize buffer");

        if let Some(proc_rpt_path) = &self.report {
            if let Err(why) = ha_processing.write_process_report(proc_rpt_path) {
                error!("Failed to write process report: {:?}", why);
            }
        }
        println!("Process Report: \n{}", ha_processing.process_report);
    }
}
