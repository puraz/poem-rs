#[cfg(unix)]
pub fn install_known_warning_filter() {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};
    use std::os::fd::{FromRawFd, RawFd};
    use std::thread;

    const SUPPRESSED: &str = "ICU4X data error: No segmentation model for language: ja";

    unsafe {
        let mut pipe_fds: [RawFd; 2] = [0; 2];
        if libc::pipe(pipe_fds.as_mut_ptr()) != 0 {
            return;
        }

        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];
        let original_stderr = libc::dup(libc::STDERR_FILENO);
        if original_stderr < 0 {
            libc::close(read_fd);
            libc::close(write_fd);
            return;
        }

        if libc::dup2(write_fd, libc::STDERR_FILENO) < 0 {
            libc::close(read_fd);
            libc::close(write_fd);
            libc::close(original_stderr);
            return;
        }

        libc::close(write_fd);

        thread::spawn(move || {
            let read_file = File::from_raw_fd(read_fd);
            let mut original = File::from_raw_fd(original_stderr);
            let reader = BufReader::new(read_file);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        if text.contains(SUPPRESSED) {
                            continue;
                        }
                        let _ = writeln!(original, "{text}");
                    }
                    Err(_) => break,
                }
            }
            let _ = original.flush();
        });
    }
}

#[cfg(not(unix))]
pub fn install_known_warning_filter() {}
