mod ini_unit;

use clap::{App, load_yaml};
use colored::*;
use rand::rngs::ThreadRng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::ffi::OsStr;
use std::io::{Write, self};
use std::path::PathBuf;
use std::fs::{create_dir_all, OpenOptions};
use std::process::exit;
use std::sync::{Mutex, Arc};
use std::time::{Instant, Duration};

use ini_unit::ini_unit::*;

macro_rules! time {
  ($time:expr) => {
      {
        if $time.elapsed() < Duration::new(1, 0) { format!("{} ms",$time.elapsed().as_millis()) } else { format!("{} s",$time.elapsed().as_secs()) }
      }
  };
}

fn main() {
  let error_text: ColoredString="[Error]".red();
  let log_text: ColoredString="[Log]".blue();
  let mut count:i32 = 0;
  let start_time = Instant::now();//运行起始时间

  let yml = load_yaml!("yaml.yml");//使用load_yaml宏读取yaml.yml内的内容
  let matches = App::from_yaml(yml).get_matches();

  let mut opath= PathBuf::from("./output");
  let root:PathBuf;

  if let Some(o) = matches.value_of("output") {//获取opath为--output参数
    opath = PathBuf::from(o);
  }

  if let Some(f) = matches.value_of("input") {//获取f为--input参数
    let path=PathBuf::from(f);
    if let Some(r) = matches.value_of("root") {//获取root为--root参数
      root = PathBuf::from(r);
    }else{
      let mut tmp = path.clone();
      if tmp.is_file(){
        tmp.pop();
      }
      root = tmp;
    }
    
    if path.is_file(){
      match Ini::load_from_file(&path){
        Ok(ini) => {
          println!("{:?}",ini);
          let mut random:ThreadRng = rand::thread_rng();
          match output(ini,&root,&opath,&mut random) {
            Ok(_) => {},
            Err(err) => println!("{}",err),
        }
          count+=1},
        Err(err) => println!("{}",err)
      }
    }else if path.is_dir() {
      count=load_dir(PathBuf::from(f),&root,&opath)
    }else {
      println!("{}输入文件不存在",error_text);
      exit(0);
    }
  }else {
    println!("{}无文件输入,请使用 rwc -h 查询使用方法",error_text);
    exit(0);
  }
    if count>0 {
        println!("{}所有文件输出完成",log_text);
        println!("共耗时{}",time!(start_time));
        println!("共处理{} 个单位",count)
    }else {
        println!("{}无文件输出",error_text);
        println!("共处理{} 个单位",count)
    }
    
}

//加载文件夹内ini
fn load_dir(f:PathBuf,root:&PathBuf,opath:&PathBuf)->i32{
  
  let count = Arc::new(Mutex::new(0));
  let _count = count.clone();
  let log_text: ColoredString="[Log]".blue();

  let mut paths:Vec<PathBuf> =vec![];
  for entry in walkdir::WalkDir::new(f){
    paths.push(entry.unwrap().path().to_path_buf())
  }
  //多线程处理
  paths.par_iter().for_each(|path|{
    
    if path.extension().eq(&Some(OsStr::new("ini"))) {
      match Ini::load_from_file(&path.to_path_buf()){
        Ok(mut ini) => {
          if let Some(s) = ini.data.get("core") {
              if s.contains_key("dont_load"){
                  println!("{}{} 含有dont_load:true，跳过此文件",log_text,path.display());
                  return;//不加载的ini 跳过
              }
          }
          match ini.load_copyfrom(root) {
              Ok(_)=>{},
              Err(err)=>{println!("{}{} :{}","[Error]".red(),ini.path.display(),err)}
          }
          let mut random:ThreadRng = rand::thread_rng();
          match output(ini, root,&opath,&mut random){
            Ok(_) => {},
            Err(err) => println!("{}",err),
        };
          *count.lock().unwrap()+=1
        },
        Err(err) => {println!("{}{}","[Error]".red(),err)}
      }
    }
  });
  let count = *count.lock().unwrap();
  count
}

fn output(mut ini:Ini,root:&PathBuf,opath:&PathBuf,random:&mut ThreadRng) -> Result<(),String>{
  let core=opath.join(get_name(opath,random).clone()+".ini");
  let data=opath.join(get_name(opath,random).clone());
  let conf=opath.join(get_name(opath,random).clone());
  let conf_path=conf.file_name().unwrap();
  let data_path=data.file_name().unwrap();
  if !opath.exists(){
    match create_dir_all(&opath){
      Ok(())=>{}
      Err(err)=>{
        return Err(format!("{}{}{}","[Error]".red(),"输出文件夹创建失败",err));
      }
    }
  }

  //创建文件
  let core_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&core);
  let data_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&data);
  let conf_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&conf);

  let (mut core_ini,conf_ini,data_ini)=ini.code(root,opath,random);
  let error_text: ColoredString="[Error]".red();

  core_ini.set_kv("core".to_string(), "copyFrom".to_string(), "".to_string()+data_path.to_str().unwrap()+","+conf_path.to_str().unwrap());

    match write_to(&core_ini,&mut core_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{return Err(format!("{}{} :{}",ini.path.display(),error_text,err))}
    };
    match write_to(&conf_ini,&mut conf_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{return Err(format!("{}{} :{}",ini.path.display(),error_text,err))}
    };
    match write_to(&data_ini,&mut data_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{return Err(format!("{}{} :{}",ini.path.display(),error_text,err))}
    };
  Ok(())
}

//输出ini到文件
fn write_to<W: Write>(ini: &Ini, writer: &mut W) -> io::Result<()> {
    for (section_name,section) in ini.data.iter(){
      //遍历节内数据
      writeln!(writer, "[{}]",section_name)?;
      for (k,v) in section.iter(){
        writeln!(writer, "{}:{}", k, v)?;
      }
    }
    Ok(())
}
