use clap::{App, load_yaml};
use colored::*;
use rand::rngs::ThreadRng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::ffi::OsStr;
use std::io::{Write, self};
use std::path::PathBuf;
use std::fs::{create_dir_all, OpenOptions, File, copy};
use std::process::exit;
use std::sync::{Mutex, Arc};
use rand::Rng;
use std::time::{Instant, Duration};
use hashbrown::*;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut};
use pcre2::bytes::*;

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

  /*let regex = Regex::new(r#"\$\{.{0,}\}"#).unwrap().captures(b"${aaaabbbbczz.aabiiilll}");
  match regex {
    Ok(Some(a)) =>println!("{}",std::str::from_utf8(a.get(0).unwrap().as_bytes()).unwrap()),
    Ok(None) => println!("NONE"),
    Err(_) => exit(0),
  }
  return;*/

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

fn get_bin(random:&mut ThreadRng) -> String{
  let i =random.gen::<u32>();
  let mut re:String=String::from("");
  let mut tmp = i;
  loop {
    if tmp==1 {
      re = String::from("1") + &re;
      break;
    }else if tmp==2 {
      re = String::from("2") + &re;
      break;
    }else {
      re = (tmp%3).to_string() + &re;
      tmp = (tmp-tmp%3)/3;
    }
  }
  return re.replace("2", "l").replace("1", "I").replace("0", "1");
}

fn get_name(opath:&PathBuf,random:&mut ThreadRng)->String{//获取一个不会重复的文件名
  let a = get_bin(random);
  if opath.join(&a).exists() {
      return get_name(opath,random);
  }else {
      return a;
  }
}

#[derive(Debug)]
pub struct Ini {
    pub path:PathBuf,
    pub ppath: PathBuf, //父文件夹
    pub data: HashMap<String, HashMap<String, String>>,
    pub refs: HashMap<String,String>
}

impl Ini {
    fn load_from_file(path: &PathBuf) -> Result<Ini, String> {
        match File::open(path) {
            Ok(file) => {
              let mut linecount = 0;
              let br = BufReader::new(file);
              let mut data: HashMap<String, HashMap<String, String>> = HashMap::new();
              let mut m = Tmp {
                mode: Mode::COM,
                st: String::from(""),
                section_name: String::from(""),
                section: HashMap::new(),
                stname: String::new(),
                refs: HashMap::new()
              }; //暂存原始字符串
              for l in br.lines() {
                match l {
                  Ok(line) => {
                    use Mode::*;
                      match &m.mode {
                        COM => {//普通
                          use LineType::*;
                          match m.gettype(&line) {
                            STR => {
                              linecount+=1;
                            }
                            KV => {
                              if !m.getsname().eq("") {
                                let (k,v) = line.split_once(":").unwrap();
                                if v.starts_with("\"\"\"") {//开始
                                  if v.len() >= 6 && v.ends_with("\"\"\"") {
                                    m.addkv(k.to_string(), v.to_string())
                                  } else {
                                    m.turn();
                                    m.setstrname(k.to_string());
                                    m.setstr(v.to_string()) //开始记录原始字符串
                                  }
                                } else {
                                    m.addkv(k.to_string(), v.to_string());
                                }
                                linecount+=1;
                              }else {
                                return Err(format!("{}{} :第{}行格式错误: {}","[Error]".red(),path.display(),linecount,line));
                              }
                            }
                            SECTION => {
                            //此行是section
                              if !&m.getsname().is_empty() {
                              //此前存在section
                                data.insert(m.getsname(), m.getsection());
                              }
                                m.clearsection();
                                m.setsname(line[1..line.len() - 1].to_string());
                                linecount+=1;
                            }
                            EMPTY => {linecount+=1;}
                            UNKNOW => {
                              linecount+=1;
                              return Err(format!("{}{} :第{}行格式错误: {}","[Error]".red(),path.display(),linecount,line));
                            }
                          }
                        }
                        STR => {//原始字符串记录
                          m.setstr(m.getstr() + "\n" + &line);
                          if line.ends_with("\"\"\"") {
                            m.addkv(m.getstrname(), m.getstr());
                            m.clearstr();
                            m.turn();
                          }
                          linecount+=1;
                        }
                      }
                  }
                  Err(_) => {}
                }
              }
              if !m.getsname().is_empty() {
                data.insert(m.getsname(), m.getsection());
              }
              let mut p = path.clone();
              p.pop();
              return Ok(Ini {path: path.to_path_buf(), data, ppath: p ,refs:m.refs});
            }
            Err(_) => {
                return Err(String::from(format!("打开文件 {} 失败",path.display())));
            }
      }
    }
    fn code(&mut self,root:&PathBuf,opath:&PathBuf,random:&mut ThreadRng) -> (Ini, Ini, Ini) {
        let mut core_ini = Ini::new();
        let mut conf_ini = Ini::new();
        let mut data_ini = Ini::new();
        let regex = Regex::new(r#"\$\{.{0,}\}"#).unwrap();
        for sec in &self.data{
            for (k, mut v) in sec.1 {
                //解${}引用
                if regex.is_match(v.as_bytes()).unwrap(){
                  match regex.captures(v.as_bytes()).unwrap() {
                    Some(re) => {
                      use RefType::*;
                      let s=std::str::from_utf8(re.get(0).unwrap().as_bytes()).unwrap();
                      match getrf(s) {
                        SREF => {println!("sref:{}",s)},
                        REF => {
                          v=sec.1.get(s).unwrap();
                        },
                        BDS => {println!("bds:{}",s)},
                    }
                    },
                    None => todo!(),
                }
                  //println!("{}",.get(0).unwrap().as_bytes()).unwrap())
                }
                //过滤不能使用$的键
                match &k[..] {
                    "name"
                    | "copyFrom"
                    | "altNames"
                    | "class"
                    | "overrideAndReplace"
                    | "onNewMapSpawn"
                    | "canOnlyBeAttackedByUnitsWithTags"
                    | "tags"
                    | "similarResourcesHaveTag"
                    | "displayText"
                    | "displayDescription"
                    | "displayLocaleKey"
                    | "transportUnitsRequireTag"
                    | "canReclaimResourcesOnlyWithTags"
                    | "unitsSpawnedOnDeath"
                    | "soundOnDeath"
                    | "isLockedMessage"
                    | "isLockedAltMessage"
                    | "isLockedAlt2Message"
                    | "teamColoringMode"
                    | "drawLayer"
                    | "attackMovement"
                    | "interceptProjectiles_withTags"
                    | "shoot_sound"
                    | "friendlyFire"
                    | "movementType"
                    | "upgradedFrom"
                    | "onlyUseAsHarvester_ifBaseHasUnitTagged"
                    | "priority"
                    | "stripIndex"
                    | "onActions"
                    | "text"
                    | "textPostFix"
                    | "description"
                    | "displayType"
                    | "showMessageToAllEnemyPlayers"
                    | "showQuickWarLogToPlayer"
                    | "showQuickWarLogToAllPlayers"
                    | "anyRuleInGroup"
                    | "cannotPlaceMessage"
                    | "displayName"
                    | "displayNameShort"
                    | "autoTriggerOnEvent"
                    | "fireTurretXAtGround_onlyOverPassableTileOf"
                    | "deleteNumUnitsFromTransport_onlyWithTags"
                    | "addWaypoint_target_randomUnit_team"
                    | "attachments_onlyOnSlots"
                    | "showMessageToPlayer"
                    | "showMessageToAllPlayers"
                    | "" => {
                        match core_ini.data.get_mut(sec.0) {
                            Some(s) => {
                                s.insert(k.clone(), v.clone());
                                continue;
                            }
                            None => {
                                //不存在节
                                let mut s: HashMap<String, String> = HashMap::new();
                                s.insert(k.to_string(), v.to_string());
                                core_ini.data.insert(sec.0.to_string(), s); //创建节
                                continue;
                            }
                        }
                      },
                      //将图片复制到输出路径
                      "image"|"image_wreak"|"image_turret"|"image_shadow"|"image_back"=>{
                        let image_opath = opath.join(get_name(opath,random));
                        let v=v.trim();
                        let image_path = if v.starts_with("ROOT:") {
                          root.join(v.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""))
                        }else if v.starts_with("SHARED:") |v.starts_with("SHADOW:")| v.to_uppercase().eq("NONE") | v.to_uppercase().eq("AUTO") {
                          match core_ini.data.get_mut(sec.0) {
                            Some(s) => {
                                s.insert(k.clone(), v.to_string());
                                continue;
                            }
                            None => {
                                //不存在节
                                let mut s: HashMap<String, String> = HashMap::new();
                                s.insert(k.to_string(), v.to_string());
                                core_ini.data.insert(sec.0.to_string(), s); //创建节
                                continue;
                            }
                        }
                        }else{
                          self.ppath.join(v.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""))
                        };
                        match copy(&image_path, &image_opath){
                            Ok(_) => {}
                            Err(_) => {println!("{}复制 {} 失败","[Error]".red(),image_path.display());break;}
                        }
                        match core_ini.data.get_mut(sec.0) {
                            Some(s) => {
                                s.insert(k.clone(), image_opath.file_name().unwrap().to_string_lossy().to_string());
                                continue;
                            }
                            None => {
                                //不存在节
                                let mut s: HashMap<String, String> = HashMap::new();
                                s.insert(k.to_string(), image_opath.file_name().unwrap().to_string_lossy().to_string());
                                core_ini.data.insert(sec.0.to_string(), s); //创建节
                                continue;
                            }
                        }
                      }
                      _=>{
                        let cs = get_bin(random);//随机节名
                        let ck = get_bin(random);//随机键名
                        match conf_ini.data.get_mut(sec.0) {
                            Some(conf_sec) => {
                                conf_sec
                                    .insert(k.clone(), String::from("${") + &cs + "." + &ck + "}");
                            }
                            None => {
                                let mut s: HashMap<String, String> = HashMap::new();
                                s.insert(k.clone(), String::from("${") + &cs + "." + &ck + "}");
                                conf_ini.data.insert(sec.0.to_string(), s); //创建节
                            }
                        }
                        match data_ini.data.get_mut(&cs) {
                          Some(data_sec) => {
                              data_sec.insert(ck.clone(), v.clone());
                              continue;
                          }
                          None => {
                              let mut s: HashMap<String, String> = HashMap::new();
                              s.insert(ck.clone(),v.clone());
                              data_ini.data.insert(cs,s); //创建节
                              continue;
                          }
                      }
                    }
                }
            }
        };
        (core_ini, conf_ini, data_ini)
    }

    fn load_copyfrom(&mut self,root:&PathBuf)->Result<(), String> {
      let sec;
      if let Some(s)=self.data.get("core") {
        sec=s;
      }else {
          return Ok(());
      }
        if sec.contains_key("copyFrom") {
          let copy_from=self["core".to_string()]["copyFrom"].split(",");
          let mut total_ini = Ini::new();
          for path in copy_from {
            let input:PathBuf;
            let mut tmp = String::from(path);
            tmp=tmp.trim().to_string();
            if tmp.starts_with("ROOT:") {
              input = root.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
            }else{
              input = self.ppath.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
            }
            total_ini.ppath=self.ppath.clone();
              match Ini::load_from_file(&input) {
                Ok(ini) => {
                  total_ini.ppath=ini.ppath.clone();
                    for (sname,sec) in &ini.data{
                        for (k,v) in sec{
                            total_ini.set_kv(sname.clone(), k.clone(), v.clone());
                        }
                    }
                },
                Err(err) => {return Err(err);},
            }
            match total_ini.data.get("core") {
                Some(sec) => {
                  if sec.contains_key("copyFrom") {
                    match total_ini.load_copyfrom(root) {
                      Ok(_) => {},
                      Err(err) => {
                        return Err(err);
                      },
                    }
                  total_ini.data.get_mut("core").unwrap().remove("coopyFrom");
                  }
                },
                None => {},
            }
          }
          for (sname,sec) in total_ini.data{
            for (k,v) in sec{
              self.set_kv(sname.clone(), k.clone(), v.clone());
            }
          }
          self["core".to_string()].remove("dont_load");
          return Ok(());
        }else{
          self["core".to_string()].remove("dont_load");
          return Ok(())
        };
    }
    fn set_kv(&mut self, name: String, k: String, v: String) {
      match self.data.get_mut(&name) {
        Some(sec) => {
          sec.insert(k, v);
        },
        None => {
          let mut s:HashMap<String,String>=HashMap::new();
          s.insert(k, v);
          self.data.insert(name, s);
        },
      }
    }
    fn new() -> Ini {
        Ini {
            path:PathBuf::new(),
            ppath: PathBuf::new(),
            data: HashMap::new(),
            refs: HashMap::new()
        }
    }
}

impl Index<String> for Ini {
    type Output = HashMap<std::string::String, std::string::String>;
    fn index(&self, section_name: String) -> &Self::Output {
        &self.data[&section_name]
    }
}
impl IndexMut<String> for Ini {
    fn index_mut(&mut self, index: String) -> &mut Self::Output {
        self.data.get_mut(&index).unwrap()
    }
}

enum LineType {
    STR,
    KV,
    SECTION,
    EMPTY,
    UNKNOW,
}

enum Mode {
    COM,
    STR,
}

struct Tmp {
    mode: Mode,
    st: String,
    stname: String,
    section_name: String,
    section: HashMap<String, String>,
    refs: HashMap<String, String>
}

impl Tmp {

  fn gettype(&self,line: &String) -> LineType {
    let line = line.trim();
      if line.starts_with("[") && line.ends_with("]") {
          LineType::SECTION
      } else if line.contains(":")&&!line.ends_with(":") {
          LineType::KV
      } else if line.is_empty() {
          LineType::EMPTY
      } else if line.ends_with("\"\"\"")||line.starts_with("#")||self.getsname().starts_with("comment_") {
          LineType::STR
      } else {
          LineType::UNKNOW
      }
  }

    fn turn(&mut self) {
        match self.mode {
            Mode::COM => self.mode = Mode::STR,
            Mode::STR => self.mode = Mode::COM,
        }
    }
    fn getstr(&self) -> String {
        self.st.clone()
    }
    fn setstr(&mut self, s: String) {
        self.st = s
    }
    fn setsname(&mut self, s: String) {
        self.section_name = s
    }
    fn getsname(&self) -> String {
        self.section_name.clone()
    }
    fn addkv(&mut self, k: String, v: String) {
        self.section.insert(k, v);
    }
    fn getsection(&self) -> HashMap<String, String> {
        self.section.clone()
    }
    fn setstrname(&mut self, sname: String) {
        self.stname = sname
    }
    fn getstrname(&self) -> String {
        self.stname.clone()
    }
    fn clearsection(&mut self) {
        self.section = HashMap::new();
        self.section_name = String::new();
    }
    fn clearstr(&mut self) {
        self.st = String::new();
        self.stname = String::new();
    }
}

enum RefType {
    SREF,//跨节引用
    REF,//纯纯的引用
    BDS//表达式
}

fn getrf(s:&str)->RefType {
    if s.contains("+")||s.contains("-")||s.contains("*")||s.contains("/") {
        RefType::BDS
    }else if s.contains(".") {
        RefType::SREF
    }else {
        RefType::REF
    }
}
