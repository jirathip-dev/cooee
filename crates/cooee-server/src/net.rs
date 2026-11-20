use std::{
    fmt::Write as _,
    fs,
    io::{BufRead, BufReader, Lines, Write as _},
    path::PathBuf,
    process::{Child, ChildStdout, Stdio},
};

use serde_json::Value;

pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: String,
}

impl Reply {
    pub(crate) fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub(crate) fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_default()
    }
}

fn quoted(v: &str) -> Result<String, String> {
    if v.contains(char::is_control) {
        return Err("a request field has a control character".to_owned());
    }
    Ok(format!(
        "\"{}\"",
        v.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

#[derive(Default)]
struct Spool(Vec<PathBuf>);

impl Spool {
    fn file(&mut self) -> Result<PathBuf, String> {
        let name = cooee_core::random(16).map_err(|e| e.to_string())?;
        let path = std::env::temp_dir().join(format!("cooee-body-{}", cooee_core::sha256(&name)));
        self.0.push(path.clone());
        Ok(path)
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        for p in &self.0 {
            fs::remove_file(p).ok();
        }
    }
}

fn request(
    config: &mut String,
    (method, url): (&str, &str),
    headers: &[String],
    body: Option<&[u8]>,
    spool: &mut Spool,
) -> Result<(), String> {
    writeln!(
        config,
        "url = {}\nrequest = {}\nconnect-timeout = 10\nproto = \"=https,http\"",
        quoted(url)?,
        quoted(method)?
    )
    .ok();
    for h in headers {
        writeln!(config, "header = {}", quoted(h)?).ok();
    }
    match body.map(|b| std::str::from_utf8(b).map(quoted)) {
        None => {}
        Some(Ok(Ok(text))) => writeln!(config, "data-raw = {text}").unwrap_or_default(),
        Some(_) => {
            let path = spool.file()?;
            cooee_core::write_private(&path, body.unwrap_or_default())
                .map_err(|e| e.to_string())?;
            writeln!(
                config,
                "data-binary = {}",
                quoted(&format!("@{}", path.display()))?
            )
            .ok();
        }
    }
    Ok(())
}

fn curl(config: &str) -> Result<Child, String> {
    let mut c = cooee_core::runner::command("curl")
        .args(["-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    if let Some(mut stdin) = c.stdin.take() {
        stdin
            .write_all(config.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(c)
}

fn start(
    (method, url): (&str, &str),
    headers: &[String],
    body: Option<&[u8]>,
    extra: &str,
) -> Result<(Child, Spool), String> {
    let (mut config, mut spool) = ("silent\nshow-error\n".to_owned(), Spool::default());
    request(&mut config, (method, url), headers, body, &mut spool)?;
    config.push_str(extra);
    Ok((curl(&config)?, spool))
}

pub(crate) type Post = (String, Vec<String>, Vec<u8>);

pub(crate) fn post_all(
    posts: &[Post],
    max_time: u64,
) -> Result<Vec<Result<Reply, String>>, String> {
    let mut got: Vec<Result<Reply, String>> = posts
        .iter()
        .map(|(url, ..)| Err(format!("{url}: no answer")))
        .collect();
    if posts.is_empty() {
        return Ok(got);
    }
    let mut config = format!(
        "parallel\nparallel-max = {}\nsilent\n",
        posts.len().min(300)
    );
    let (mut spool, mut outs) = (Spool::default(), Vec::new());
    for (i, (url, headers, body)) in posts.iter().enumerate() {
        if i > 0 {
            config.push_str("next\n");
        }
        request(&mut config, ("POST", url), headers, Some(body), &mut spool)?;
        let out = spool.file()?;
        writeln!(
            config,
            "max-time = {max_time}\noutput = {}\nwrite-out = \"%{{urlnum}} %{{http_code}} %{{exitcode}} %{{errormsg}}\\n\"",
            quoted(&out.display().to_string())?
        )
        .ok();
        outs.push(out);
    }
    let out = curl(&config)?
        .wait_with_output()
        .map_err(|e| e.to_string())?;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut f = line.splitn(4, ' ');
        let (Some(Ok(n)), Some(Ok(status)), Some(exit)) = (
            f.next().map(str::parse::<usize>),
            f.next().map(str::parse::<u16>),
            f.next(),
        ) else {
            continue;
        };
        let (Some(slot), Some(path), Some((url, ..))) = (got.get_mut(n), outs.get(n), posts.get(n))
        else {
            continue;
        };
        *slot = match exit {
            "0" => Ok(Reply {
                status,
                body: fs::read_to_string(path).unwrap_or_default(),
            }),
            _ => Err(format!("{url}: {}", f.next().unwrap_or_default())),
        };
    }
    Ok(got)
}

pub(crate) fn fetch(
    method: &str,
    url: &str,
    headers: &[String],
    body: Option<&[u8]>,
) -> Result<Reply, String> {
    let extra = "max-time = 20\nwrite-out = \"\\n%{http_code}\"\n";
    let (c, _spool) = start((method, url), headers, body, extra)?;
    let out = c.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", &text));
    match code.parse() {
        Ok(status) if out.status.success() => Ok(Reply {
            status,
            body: body.to_owned(),
        }),
        _ => Err(format!(
            "{url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

pub(crate) struct Stream {
    pub(crate) status: u16,
    head: Vec<String>,
    lines: Lines<BufReader<ChildStdout>>,
    child: Child,
    _spool: Spool,
}

impl Stream {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.head.iter().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case(name).then(|| v.trim())
        })
    }
}

impl Iterator for Stream {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.lines
            .next()?
            .ok()
            .map(|l| l.trim_end_matches('\r').to_owned())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

pub(crate) fn stream(
    method: &str,
    url: &str,
    headers: &[String],
    body: Option<&[u8]>,
) -> Result<Stream, String> {
    let (mut child, spool) = start((method, url), headers, body, "include\nno-buffer\n")?;
    let out = child.stdout.take().ok_or("curl: no pipe")?;
    let mut lines = BufReader::new(out).lines();
    let mut head: Vec<String> = Vec::new();
    for l in lines.by_ref().map_while(Result::ok) {
        let l = l.trim_end_matches('\r');
        match l {
            "" if head.first().is_some_and(|s| s.contains(" 100 ")) => head.clear(),
            "" => break,
            l => head.push(l.to_owned()),
        }
    }
    let status = head.first().and_then(|s| s.split(' ').nth(1)?.parse().ok());
    let Some(status) = status else {
        let mut why = String::new();
        if let Some(mut e) = child.stderr.take() {
            std::io::Read::read_to_string(&mut e, &mut why).ok();
        }
        child.kill().ok();
        child.wait().ok();
        return Err(format!("{url}: {}", why.trim()));
    };
    Ok(Stream {
        status,
        head,
        lines,
        child,
        _spool: spool,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    fn serve(answer: &'static str) -> (String, thread::JoinHandle<Vec<u8>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", l.local_addr().unwrap());
        let seen = thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut got = vec![0; 8192];
            let n = s.read(&mut got).unwrap();
            s.write_all(answer.as_bytes()).unwrap();
            got.truncate(n);
            got
        });
        (url, seen)
    }

    #[test]
    fn a_redirect_is_answered_not_followed() {
        let (url, seen) = serve(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/elsewhere\r\nContent-Length: 0\r\n\r\n",
        );
        let s = stream("GET", &url, &["Authorization: Bearer k".to_owned()], None).unwrap();
        assert_eq!(
            (s.status, s.header("location")),
            (302, Some("http://127.0.0.1:9/elsewhere"))
        );
        let got = String::from_utf8(seen.join().unwrap()).unwrap();
        assert!(got.contains("Authorization: Bearer k"), "{got}");
        let (url, _) = serve(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:9/x\r\nContent-Length: 0\r\n\r\n",
        );
        assert_eq!(fetch("POST", &url, &[], Some(b"{}")).unwrap().status, 307);
    }

    #[test]
    fn a_binary_body_arrives_whole() {
        let (url, seen) = serve("HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nok");
        let body = [0_u8, 159, 146, 150, b'"', b'\n'];
        let r = fetch("POST", &url, &[], Some(&body)).unwrap();
        assert_eq!((r.status, r.body.as_str()), (201, "ok"));
        assert!(seen.join().unwrap().ends_with(&body));
    }
}
