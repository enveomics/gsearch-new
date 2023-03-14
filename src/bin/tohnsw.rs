// GSEARCH v0.1.0
// Copyright 2021-2022, Jean Pierre Both and Jianshu Zhao.
// Licensed under the MIT license (http://opensource.org/licenses/MIT).
// This file may not be copied, modified, or distributed except according to those terms.




//! tohnsw --dir [-d] dir --sketch [-s] size --nbng [-n] nb --ef m [--seq]
//! 
//! --dir : the name of directory containing tree of DNA files or Amino Acid files. 
//!   
//! --sketch gives the size of probminhash sketch (integer value). Mandatory value.  
//! 
//! --algo specifiy the sketching algorithm to be used. Default is ProbMinhash. SuperMinHash or SuperMinHash2 can specified by --algo super or --algo super2.
//!         passing --algo prob   for asking ProbMinhash is possible
//! 
//! --kmer [-k] gives the size of kmer to use for generating probminhash (integer value). Mandatory argument. 
//!  
//! --nbng [-n] gives the number of neihbours required in hnsw construction at each layer, in the range 24-64 is usual
//!             it doest not means you cannot ask for more neighbours in request.
//! 
//!  --ef optional integer value to optimize hnsw structure creation (default to 400)  
//! 
//!  --seq if we want a processing by sequences. Default is to concatenate all sequneces in a file
//!             in a large sequence.
//! 
//!  --aa : set if data to process are Amino Acid sequences. Default is DNA
//! 
//!  --pio : option to read compressed files and then parallelize decompressing/fasta parsing. 
//!         Useful, with many cores if io lags behind hashing/hnsw insertion. to speed up io.  
//!         **Necessary to limit/custom the number of files or sequences simultanuously loaded in memory if files are very large (tens of Gb)**.  
//! 
//! --add : This option is dedicated to adding new data to a hnsw structure.  
//!         The program reloads a previous dump of the hnsw structures. tohnsw must (presently) be launched from the directory
//!         containing the dump as the program looks for the files "hnswdump.hnsw.data" and "hnswdump.hnsw.graph" created previously.  
//!         **In this case parameters corresponding to options --kmer  --sketch --nbng --ef and --algo are reloaded from file parameters.json**.  
//!         It is useless to pass them in command line.

// must loop on sub directories , open gzipped files
// extracts complete genomes possiby many in one file (get rid of capsid records if any)
// compute probminhash sketch and store in a Hnsw.

// one thread should read sequences and do the probminhash
// another process should store in hnsw

// hnsw should also run in a query server mode after insertion.

use clap::{Command, Arg};

use std::path::Path;

// for logging (debug mostly, switched at compile time in cargo.toml)
use env_logger::{Builder};

//


// our crate

use gsearch::dna::dnasketch::dna_process_tohnsw;
use gsearch::aa::aasketch::aa_process_tohnsw;
use gsearch::utils::*;

//=========================================================================

// install a logger facility
pub fn init_log() -> u64 {
    Builder::from_default_env().init();
    println!("\n ************** initializing logger *****************\n");    
    return 1;
}

// this function does the sketching in small kmers (less than 14 bases) and hnsw store of a whole directory, version before generic one

fn main() {
    let _ = init_log();
    //
    //
    let matches = Command::new("tohnsw")
        .arg(Arg::new("dir")
            .long("dir")
            .short('d')
            .takes_value(true)
            .required(true)
            .help("name of directory containing genomes to index"))
        .arg(Arg::new("kmer_size")
            .long("kmer")
            .short('k')
            .required(true)
            .takes_value(true)
            .help("expecting a kmer size"))
        .arg(Arg::new("sketch_size")
            .long("sketch")
            .short('s')
            .required(true)
            .takes_value(true)
            .help("size of probminhash sketch, default to 256"))
        .arg(Arg::new("sketch_algo")
            .long("algo")
            .takes_value(true)
            .default_value("prob")
            .help("specify algo for sketching, prob for probminhash or super for superminhash"))
        .arg(Arg::new("neighbours")
            .long("nbng")
            .short('n')
            .required(true)
            .takes_value(true)
            .help("must specify number of neighbours in hnsw"))
        .arg(Arg::new("ef")
            .long("ef")
            .default_value("400")
            .help("parameters neighbour search at creation"))
        .arg(Arg::new("aa")
            .help("to specify amino acid seq processing")
            .long("aa")
            .takes_value(false))
        .arg(Arg::new("seq")
            .long("seq")
            .takes_value(false)
            .help("--seq to get a processing by sequence"))
        .arg(Arg::new("pario")
            .long("pio")
            .takes_value(true)
            .help("--pio nfiles"))
        .arg(Arg::new("add")
            .long("add")
            .takes_value(false)
            .help("--add to add sequence entries from a directory into hnsw")
        )
        .get_matches();
    //
    // by default we process DNA files in one large sequence block
    let mut block_processing = true;
    // decode matches, check for dir
        let datadir;
        if matches.is_present("dir") {
            println!("decoding argument dir");
            datadir = matches.value_of("dir").ok_or("").unwrap().parse::<String>().unwrap();
            if datadir == "" {
                println!("parsing of dir failed");
                std::process::exit(1);
            }
        }
        else {
            println!("-d dirname is mandatory");
            std::process::exit(1);
        }
        let dirpath = Path::new(&datadir);
        if !dirpath.is_dir() {
            println!("error not a directory : {:?}", datadir);
            std::process::exit(1);
        }
        //
        // get sketching params
        //
        let sketch_size;
        if matches.is_present("sketch_size") {
            sketch_size = matches.value_of("sketch_size").ok_or("").unwrap().parse::<u16>().unwrap();
            println!("sketching size arg {}", sketch_size);
        }
        else {
            panic!("sketch_size is mandatory");
        }
        // sketching algorithm
        let mut sketch_algo = SketchAlgo::PROB3A;
        if matches.is_present("sketch_algo") {
            let algo = matches.value_of("sketch_algo").ok_or("").unwrap().parse::<String>().unwrap();
            println!("sketching algo {}", algo);
            if algo == String::from("super") {
                sketch_algo = SketchAlgo::SUPER;
            }
            else if algo == String::from("super2") {
                sketch_algo = SketchAlgo::SUPER2;
            }
            else if algo == String::from("prob") {
                sketch_algo = SketchAlgo::PROB3A;
            }
            else {
                println!("unknown asketching algo");
                std::panic!("unknown asketching algo");
            }
        }
        // kmer size
        let kmer_size;
        if matches.is_present("kmer_size") {
            kmer_size = matches.value_of("kmer_size").ok_or("").unwrap().parse::<u16>().unwrap();
            println!("kmer size {}", kmer_size);
        }
        else {
            panic!(" kmer size is mandatory");
        }
        let sketch_params =  SeqSketcherParams::new(kmer_size as usize, sketch_size as usize, sketch_algo);  
        //
        let nbng;
        if matches.is_present("neighbours") {
            nbng = matches.value_of("neighbours").ok_or("").unwrap().parse::<u16>().unwrap();
            println!("nb neighbours you will need in hnsw requests {}", nbng);
        }        
        else {
            println!("-n nbng is mandatory");
            std::process::exit(1);
        }
        //
        let mut ef_construction = 400;
        if matches.is_present("ef") {
            ef_construction = matches.value_of("ef").ok_or("").unwrap().parse::<usize>().unwrap();
            println!("ef construction parameters in hnsw construction {}", ef_construction);
        }
        else {
            println!("ef default used in construction {}", ef_construction);
        }
        // do we use block processing, recall that default is yes
        if matches.is_present("seq") {
            println!("seq option , will process every sequence independantly ");
            block_processing = false;
        }
        //
        let data_type;
        if matches.is_present("aa") {
            println!("data to processs are AA data ");
            data_type = DataType::AA;
        }
        else {
            println!("data to processs are DNA data ");
            data_type = DataType::DNA;            
        }
        // now we fill other parameters : parallel fasta parsing and adding mode in hnsw
        let nb_files_par : usize;
        if matches.is_present("pario") {
            nb_files_par = matches.value_of("pario").ok_or("").unwrap().parse::<usize>().unwrap();
            println!("parallel io, nb_files_par : {}", nb_files_par);
        }
        else {
            nb_files_par = 0;
        }
        // adding sequences
        let addseq: bool;
        if matches.is_present("add") {
            println!("adding sequences");
            addseq = true;
        }
        else {
            addseq = false;
        }
        let other_params = ComputingParams::new(nb_files_par, addseq);
        // We have everything   
        // max_nb_conn must be adapted to the number of neighbours we will want in searches.
        
        // Maximum allowed nbng for hnswlib-rs is 256. Larger nbng will not work and default to 256.
        let max_nb_conn : u8 = 255.min(nbng as u8);
        let hnswparams = HnswParams::new(1_500_000, ef_construction, max_nb_conn);
        //
        // do not filter small seqs when running file in a whole block
        let filter_params = FilterParams::new(0);
        let processing_parameters = match addseq {
            true => {  
                    log::info!("reloading parameters from previous runs, in the current directory"); 
                    let cwd = std::env::current_dir().unwrap();
                    let reload_res = ProcessingParams::reload_json(&cwd);
                        if reload_res.is_ok()  {
                            reload_res.unwrap()
                        }
                    else {
                        std::panic!("cannot reload parameters (file parameters.json) from dir : {:?}", &cwd);
                    }
                },
            _ => { ProcessingParams::new(hnswparams, sketch_params, block_processing)}
        };
        //
        match data_type {
            DataType::DNA => dna_process_tohnsw(&dirpath, &filter_params, &processing_parameters, &other_params),
            DataType::AA => aa_process_tohnsw(&dirpath, &filter_params, &processing_parameters, &other_params),
        }
        //
 } // end of main
