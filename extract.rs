use std::path::Path;
use std::{
    env,
    fs::{File, OpenOptions},
};
use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
};

fn sync_back(dir: &String, markdowns: &[String]) {
    let dir = Path::new(&dir);

    for markdown in markdowns {
        let reader = BufReader::new(File::open(markdown).unwrap());
        let tmpfile = format!("{}.tmp", markdown);
        let mut writer = BufWriter::new(File::create(&tmpfile).unwrap());
        let mut in_block = false;
        let mut java_file_name: String = "".to_string();

        for line in reader.lines() {
            let line = line.expect("Failed to read line");
            if line == "```java" {
                in_block = true;
            } else if line == "```" && in_block {
                in_block = false;
                println!("{:?}", dir.join(&java_file_name));
                let mut java_file = File::open(dir.join(&java_file_name)).unwrap();
                writer.write_all(b"```java\n").unwrap();
                std::io::copy(&mut java_file, &mut writer).unwrap();
                writer.write_all(b"```\n").unwrap();
            } else if in_block && line.starts_with("public class") {
                let words: Vec<&str> = line.split_ascii_whitespace().collect();
                java_file_name = words.get(2).unwrap().to_string() + ".java";
            } else if !in_block {
                writer.write_all(line.as_bytes()).unwrap();
                writer.write_all(b"\n").unwrap();
            }
        }
        drop(writer);
        fs::rename(tmpfile, markdown).unwrap();
    }
}

fn consume_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let res = args.iter().position(|arg| arg == flag);
    match res {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

fn consume_opt(args: &mut Vec<String>, opt: &str) -> Option<String> {
    let res = args.iter().position(|arg| arg == opt);
    match res {
        Some(i) => {
            args.remove(i);
            Some(args.remove(i))
        }
        None => None,
    }
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();

    if let Some(dir) = consume_opt(&mut args, "-b") {
        sync_back(&dir, &args);
        return;
    }

    let out_dir_name = consume_opt(&mut args, "-o");
    if out_dir_name.is_none() {
        eprintln!("Need an output directory");
        return;
    }
    let cwd = env::current_dir().unwrap();
    let out_dir = cwd.join(Path::new(&out_dir_name.unwrap()));

    for file in args {
        println!("Scanning {}", file);
        let input_path = Path::new(&file);

        // Open the input file for reading
        let file = File::open(input_path).expect("Failed to open file");
        let reader = BufReader::new(file);

        let mut current_block: String = String::new();
        let mut in_block = false;
        let mut java_file_name: String = "".to_string();

        for line in reader.lines() {
            let line_str = line.expect("Failed to read line");

            if line_str == "```java" {
                // Start a new block
                in_block = true;
            } else if line_str == "```" {
                // End current block and write it to a file
                if in_block {
                    write_block(&current_block, out_dir.join(&java_file_name).as_path());
                }
                current_block.clear();
                in_block = false;
            } else if in_block {
                // Append line to current block
                if line_str.starts_with("public class ") {
                    let words: Vec<&str> = line_str.split_ascii_whitespace().collect();
                    java_file_name = words.get(2).unwrap().to_string() + ".java";
                }
                current_block.push_str(&line_str);
                current_block.push('\n');
            }
        }
    }
}

fn write_block(block: &str, output_path: &Path) {
    fs::create_dir_all(output_path.parent().unwrap()).expect("Failed to create parent directories");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&output_path)
        .expect("Failed to create output file");

    file.write_all(block.as_bytes())
        .expect("Failed to write to file");
    println!("Created {:?}", output_path);
}
