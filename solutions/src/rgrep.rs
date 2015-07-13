use std::io::prelude::*;
use std::{io, fs, thread, process};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::Arc;

#[derive(Clone,Copy)]
enum OutputMode {
    Print,
    SortAndPrint,
    Count,
}
use self::OutputMode::*;

struct Options {
    files: Vec<String>,
    pattern: String,
    output_mode: OutputMode,
}

struct Line {
    data: String,
    file: usize,
    line: usize,
}

fn read_files(options: Arc<Options>, out_channel: SyncSender<Line>) {
    for (fileidx, file) in options.files.iter().enumerate() {
        let file = fs::File::open(file).unwrap();
        let file = io::BufReader::new(file);
        for (lineidx, line) in file.lines().enumerate() {
            let line = Line { data: line.unwrap(), file: fileidx, line: lineidx };
            out_channel.send(line).unwrap();
        }
    }
}

fn filter_lines(options: Arc<Options>, in_channel: Receiver<Line>, out_channel: SyncSender<Line>) {
    for line in in_channel.iter() {
        if line.data.contains(&options.pattern) {
            out_channel.send(line).unwrap();
        }
    }
}

fn output_lines(options: Arc<Options>, in_channel: Receiver<Line>) {
    match options.output_mode {
        Print => {
            for line in in_channel.iter() {
                println!("{}:{}: {}", options.files[line.file], line.line, line.data);
            }
        },
        Count => {
            let count = in_channel.iter().count();
            println!("{} hits for {}.", count, options.pattern);
        },
        SortAndPrint => {
            let _data: Vec<Line> = in_channel.iter().collect();
            unimplemented!()
        }
    }
}

static USAGE: &'static str = "
Usage: rgrep [-c] [-s] <pattern> <file>...

Options:
    -c, --count  Count number of matching lines (rather than printing them).
    -s, --sort   Sort the lines before printing.
";

fn get_options() -> Options {
    use docopt::Docopt;

    // Parse argv and exit the program with an error message if it fails.
    let args = Docopt::new(USAGE).and_then(|d| d.parse()).unwrap_or_else(|e| e.exit());
    let count = args.get_bool("-c");
    let sort = args.get_bool("-s");
    let pattern = args.get_str("<pattern>");
    let files = args.get_vec("<file>");
    if count && sort {
        println!("Setting both '-c' and '-s' at the same time does not make any sense.");
        process::exit(1);
    }

    // We need to make the strings owned to construct the `Options` instance.
    Options {
        files: files.iter().map(|file| file.to_string()).collect(),
        pattern: pattern.to_string(),
        output_mode: if count { Count } else if sort { SortAndPrint } else { Print },
    }
}

fn run(options: Options) {
    let options = Arc::new(options);

    // Set up the chain of threads. Use `sync_channel` with buffer-size of 16 to avoid needlessly filling RAM.
    let (line_sender, line_receiver) = sync_channel(16);
    let (filtered_sender, filtered_receiver) = sync_channel(16);

    let options1 = options.clone();
    let handle1 = thread::spawn(move || read_files(options1, line_sender));
    let options2 = options.clone();
    let handle2 = thread::spawn(move || filter_lines(options2, line_receiver, filtered_sender));
    let options3 = options.clone();
    let handle3 = thread::spawn(move || output_lines(options3, filtered_receiver));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

pub fn main() {
    run(get_options());
}
