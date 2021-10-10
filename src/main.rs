extern crate notify;

use notify::{Watcher, RecursiveMode, RawEvent,raw_watcher};
//use std::collections::HashMap;
//use std::iter::Map;
//use std::sync::{Mutex,Arc};
//use std::error::Error;
use std::sync::mpsc::channel;
use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::Session;
use std::io::stdin;
use std::path::Path;
use std::path::PathBuf;
use std::fs::File;
use indicatif::{ProgressBar,ProgressStyle};
use chrono::prelude::*;
use std::{ thread};
struct DirOp{
    l:PathBuf,
    r:PathBuf,
    t:i64
}
fn main() {
    let (addr,username,password) = ssh_auth();
    let (tx,rx)=channel();
    //let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();
    let mut watcher = raw_watcher(tx).unwrap();
    println!("(本地)notify dir:");
    let mut notify_dir = String::new();
    let mut upload_dir: String =String::new();
    stdin().read_line(&mut notify_dir).unwrap();
    println!("(服务器)upload dir:");
    stdin().read_line(&mut upload_dir).unwrap();
    watcher.watch(notify_dir.trim(), RecursiveMode::Recursive).unwrap();
    let loca_path_buf: PathBuf = PathBuf::from(notify_dir.trim());
    let (f_tx,f_rx) = channel();
    let f_send=f_tx.clone();
    let f_addr = addr.clone();
    let f_username=username.clone();
    let f_password=password.clone();
    thread::spawn(move||{
        loop {
            match rx.recv() {
                Ok(RawEvent{path:Some(path),op:Ok(op),cookie}) =>{
                    //let fname = path.as_path().file_name().unwrap().to_os_string();
                    let lit = path.ancestors();
                    let remote_file: PathBuf = dir_filter(&upload_dir.trim().to_string(),lit,&loca_path_buf);
                    
                   // let ff =format!("{}", format!("{:?}",fname)
                    //            .trim_start_matches(|c| c=='"' || c=='\'')
                    //            .trim_end_matches(|c| c=='"' || c=='\''));
                    
                    if op.contains(notify::op::WRITE) || op.contains(notify::op::CREATE) {
                        if path.is_dir() {
                            let cmd=format!("mkdir -p {}",remote_file.display().to_string());
                            ssh_cmd(&f_addr, &f_username, &f_password, &cmd);
                        }else if path.is_file() {
                            let location_now = Local::now();
                            let mills = location_now.timestamp_millis();//毫秒
                            let wdir=DirOp{ l: path.to_path_buf(), r: remote_file.to_path_buf(), t: mills };
                            f_send.send(wdir).unwrap();
                          //ssh_upload(&addr, &username, &password, remote_file.to_path_buf(), &path.as_path().to_path_buf());
                        }else {
                            let cmd=format!("mkdir -p {}",remote_file.display().to_string());
                            ssh_cmd(&f_addr, &f_username, &f_password, &cmd);
                        }
                    }
                    else if op.contains(notify::op::RENAME) || op.contains(notify::op::REMOVE) {
                        if !path.exists() { 
                          let cmd=format!("rm -rf {}",remote_file.display().to_string());
                          ssh_cmd(&f_addr, &f_username, &f_password, &cmd);
                        }else if path.is_file(){
                            let location_now = Local::now();
                            let mills = location_now.timestamp_millis();//毫秒
                            let wdir=DirOp{ l: path.to_path_buf(), r: remote_file.to_path_buf(), t: mills };
                            f_send.send(wdir).unwrap();
                            //ssh_upload(&addr, &username, &password, remote_file.to_path_buf(), &path.as_path().to_path_buf());
                        }else {
                            let cmd=format!("mkdir -p {}",remote_file.display().to_string());
                            ssh_cmd(&f_addr, &f_username, &f_password, &cmd);
                        }
                    }
                    println!("{:?} {:?} ({:?})",op,path,cookie)
                },
               Ok(event) => println!("{:?}", event),
               Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
    //  let wmaps=Arc::new(Mutex::new(HashMap::new()));
    //  let cmu_map=Arc::clone(&wmaps);
    //  //let ff_send = f_tx.clone();
    //  thread::spawn(move || {
    //      loop{
    //         (*cmu_map).lock().unwrap().iter().for_each(  |d: (&String, &DirOp)| {
    //             let wdir=DirOp{ l: d.1.l.clone(), r: d.1.r.clone(), t: d.1.t };
    //             f_tx.send(wdir).unwrap();
    //             (*cmu_map).lock().unwrap().remove(d.0);
    //         });
            
    //      }
    // });
    loop{
        match f_rx.try_recv() {
        Ok(dop)=>{
            let location_now = Local::now();
            if 3000<=(location_now.timestamp_millis()-dop.t)  {
                ssh_upload(&addr, &username, &password, dop.r, &dop.l);
            }else{
                f_tx.send(dop).unwrap();
                // let mut mu_map=wmaps.lock().unwrap();
                // mu_map.insert(dop.l.display().to_string(), dop);
            }
        },
        Err(_)=>{}
        }
    }
}
///
/// 执行一条命令
/// 
fn ssh_cmd(addr:&str,username:&str,password:&str,cmd:&str) {
    let tcp = TcpStream::connect(addr).expect("Couldn't connect to the server...");
    
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password(&username.trim(),&password.trim()).unwrap();
    assert!(sess.authenticated());
    match sess.channel_session(){
        Ok(mut channel)=>{
            channel.exec(cmd).unwrap();
            let mut s = String::new();
            channel.read_to_string(&mut s).unwrap();
            println!("cmd: {} status: {}",cmd,s);
            channel.wait_close().unwrap();
            channel.exit_status().unwrap();
        },
        Err(error)=>println!("ssh_cmd error:{}",error),
    };
    
    
}
///
/// 认证ssh服务器
/// 
fn ssh_auth<'a>() ->(String,String,String) {
    let mut add = String::new();
    let mut username = String::new();
    let mut password = String::new();
    println!("add host:");
    stdin().read_line(&mut add).unwrap();
    println!("user name:");
    stdin().read_line(&mut username).unwrap();
    println!("password:");
    stdin().read_line(&mut password).unwrap();
//     let mut pwd=[0];
   
//     while  pwd[0]!=10 {
//         print!("\x1b[H\x1b[k");
//    if let Ok(_) = stdin().lock().read(&mut pwd[..]) {
        
//         //println!("{:?}",pwd);
//         //if pwd[0]!=10{
//         let ch = std::str::from_utf8(&pwd).unwrap();
//         for c in ch.chars().into_iter(){
//          password.push(c);
//         }
       
//      }else{
//         // println!("enter Error!");
//          break;
//      }
//      //print!("\x1b[2J");
//  }

    //stdin().read_line(&mut password).unwrap();
    println!("ssh {}@{} {}",username.trim(),add.trim(),password.trim());
    let tcp = TcpStream::connect(add.trim()).expect("Couldn't connect to the server...");
    
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password(&username.trim(),&password.trim()).unwrap();
    assert!(sess.authenticated());
    match sess.channel_session() {
        Ok(mut channel)=>{
            channel.exec("free -m").unwrap();
            let mut s = String::new();
            channel.read_to_string(&mut s).unwrap();
            println!("{}",s);
            channel.wait_close().unwrap();
            println!("{}",channel.exit_status().unwrap()); 
        },
        Err(error)=>println!("ssh_auth error:{}",error),
    };
    (add.trim().to_string(),username.trim().to_string(),password.trim().to_string())
}
///
/// 从本地上传文件到远程服务器
/// 
fn ssh_upload(addr:&str,username:&str,password:&str,remote_path:PathBuf,loca_file:&PathBuf){
    
    let tcp = TcpStream::connect(addr).expect("Couldn't connect to the server...");
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password(username,password).unwrap();
    let mut file_in = File::open(loca_file.as_path().display().to_string()).unwrap();
    let len = file_in.metadata().unwrap().len(); //file_in.stream_len().unwrap();
    let pb = ProgressBar::new(len);
    match sess.scp_send(Path::new(&remote_path.display().to_string()),
                                                0o644,len,None){
                                Ok(mut remote_file)=>{
                                    let mut buffer = [0u8; 4096];
                                    let mut up_size :u64 =0;
                                    
                                    pb.set_style(ProgressStyle::default_bar()
                                                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes_per_sec}|{bytes}/{total_bytes} ({eta})")
                                                .progress_chars("#>-"));
                                    //println!("upload file form {} to {}:{}",loca_file.display().to_string(),addr,remote_path.display().to_string());
                                    loop {
                                        let nbytes = file_in.read(&mut buffer).unwrap();
                                        remote_file.write(&buffer[..nbytes]).unwrap();
                                        up_size += std::cmp::min(nbytes as u64,len);
                                        pb.set_position(up_size);
                                        if nbytes < buffer.len() { 
                                            remote_file.send_eof().unwrap();
                                            remote_file.wait_eof().unwrap();
                                            break;
                                        }
                                    }
                                    pb.finish_with_message("ok");
                                    remote_file.close().unwrap();
                                    remote_file.wait_close().unwrap();
                                    println!("upload file: {} size:{} Ok!",loca_file.as_path().display().to_string(),len);
                                },
                                Err(error)=>println!("ssh_upload error:{}",error),
                };
}
///
/// 保留不相同的目录名称，返回新的PathBuf
/// 
fn dir_filter(root:&String,lit:std::path::Ancestors,loca_path_buf:&PathBuf) -> PathBuf {
        let mut remote_file = PathBuf::new();
        //先添加根目录
        remote_file.push(Path::new(&root));
                
        let mut tmp_p = PathBuf::new();
        //再过滤每层目录,留下不同的
        for i in lit {
            if !loca_path_buf.starts_with(i) {
                tmp_p.push(i.file_name().to_owned().unwrap());
            }
        }
        for i in tmp_p.ancestors() {
            if i.file_name() !=None {
               remote_file.push(i.file_name().unwrap());
            }
        }
                
    remote_file
}

#[test]
fn test_path(){
    let path: PathBuf=PathBuf::from("/tmp/user");
    
    let  path1 =PathBuf::from("/tmp/user/tmp/file1");
    let  it = path1.ancestors();
    let mut new_path = PathBuf::new();
    new_path.push("/Users");
    let mut tmp_p =PathBuf::new();
    for i in it {
        //let fname = i.file_name();
       // println!("{:?} {:?}",path.starts_with(i),fname);
        if !path.starts_with(i) {
            tmp_p.push(i.file_name().unwrap());
        }
        
    }
    for i in tmp_p.ancestors(){
        if i.file_name() !=None {
        println!("{:?}",i.file_name().to_owned().unwrap());
        new_path.push(i.file_name().to_owned().unwrap());
        }
    }
    println!("-- {}",new_path.display());
}

#[test]
fn test_ansi(){
    print!("\x1b[2J");
    for  i in 0..256 {
        if i>0 && i%16==0 {
            print!("\n");
        }
        print!("\x1b[38;5;{}m【{}】",i,i);
    }
    print!("\x1b[32m完成\x1b[0m\n");
}

#[test]
fn test_map(){
    let mut wmaps=HashMap::new();
    wmaps.insert("one", 1);
    wmaps.insert("tow", 2);
    wmaps.insert("one", 0);
    wmaps.remove("k");
    println!("{:?}",wmaps);
}
