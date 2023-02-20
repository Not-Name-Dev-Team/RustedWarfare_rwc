use clap::{App, load_yaml};
use colored::*;
use rand::rngs::ThreadRng;
use std::ffi::OsStr;
use std::io::{Write, self};
use std::path::PathBuf;
use std::fs::{create_dir_all, OpenOptions, File};
use std::thread;
use std::time::Duration;
use rand::Rng;
use indicatif::{ProgressBar,ProgressStyle};
use std::time::Instant;
use hashbrown::*;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut};


fn main() ->() {

  let bar: ProgressBar = ProgressBar::new_spinner();
  let error_text: ColoredString="[Error]".red();
  let log_text: ColoredString="[Log]".blue();

  let start_time = Instant::now();//运行起始时间

  bar.enable_steady_tick(Duration::from_millis(1200));
  
  match ProgressStyle::default_spinner().tick_strings(&[".","..","...",]).template("{prefix:.bold.dim} {spinner:.white} {msg}"){
    Ok(sty)=> {bar.set_style(sty);}
    Err(err)=>{println!("{}",err)}
  }

  let bar_clone = bar.clone();

  thread::spawn(move || {
    loop {
        bar_clone.inc(1);
    }
    
  });

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
        Ok(ini) => output(ini, &opath),
        Err(err) => bar.println(format!("{}",err)),
    }
    }else if path.is_dir() {
      load_dir(&bar,PathBuf::from(f), root,&opath)
    }else {
      bar.println(format!("{}输入文件不存在",error_text));
      return;
    }
    bar.set_prefix("Writing");
    bar.println(format!("{}所有文件输出完成",log_text));
    bar.println(format!("共耗时{} s",start_time.elapsed().as_secs()));
    bar.finish_and_clear();
  }

  //检测ini

}

//加载文件夹内ini
fn load_dir(bar:&ProgressBar,f:PathBuf,root:PathBuf,opath:&PathBuf){
  bar.set_prefix("Reading ");
  for entry in walkdir::WalkDir::new(f){
    if entry.as_ref().unwrap().path().extension().eq(&Some(OsStr::new("ini"))) {
      bar.set_message(entry.as_ref().unwrap().path().to_str().unwrap().to_string());
      match Ini::load_from_file(&entry.as_ref().unwrap().path().to_path_buf()){
        Ok(ini) => output(ini, &opath),
        Err(err) => bar.println(format!("{}",err)),
      }
    }
  }
}

fn output(ini:Ini,opath:&PathBuf){
  let core=opath.join(get_name(opath).clone()+".ini");
  let data=opath.join(get_name(opath).clone());
  let conf=opath.join(get_name(opath).clone());
  if !opath.exists(){
    match create_dir_all(&opath){
      Ok(())=>{}
      Err(err)=>{
        println!("{}{}{}","[Error]".red(),"输出文件夹创建失败",err);
      }
    }
  }

  //创建文件
  let core_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&core);
  let data_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&data);
  let conf_file = OpenOptions::new().read(true).write(true).append(false).create(true).open(&conf);

  let (mut core_ini,conf_ini,data_ini)=ini.code();
  let error_text: ColoredString="[Error]".red();

  core_ini.set_kv("core".to_string(), "copyFrom".to_string(), "{".to_string()+data.to_str().unwrap()+","+conf.to_str().unwrap()+"}");

    match write_to(&core_ini,&mut core_file.unwrap()){
    Ok(())=>{},
      Err(err)=>{println!("{}{}",error_text,err)}
    };
    match write_to(&conf_ini,&mut conf_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{println!("{}{}",error_text,err)}
    };
    match write_to(&data_ini,&mut data_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{println!("{}{}",error_text,err)}
    };
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

fn get_bin() -> String{
  let mut random:ThreadRng = rand::thread_rng();
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

fn get_name(opath:&PathBuf)->String{//获取一个不会重复的文件名
  let a = get_bin();
  if opath.join(&a).exists() {
      return get_name(opath);
  }else {
      return a;
  }
}

#[derive(Debug)]
pub struct Ini {
    pub path:PathBuf,
    pub ppath: PathBuf, //父文件夹
    pub data: HashMap<String, HashMap<String, String>>,
}

impl Ini {
    fn load_from_file(path: &PathBuf) -> Result<Ini, String> {
        match File::open(path) {
            Ok(file) => {
              let mut linecount = 0;
                let br = BufReader::new(file);
                let mut data: HashMap<String, HashMap<String, String>> = HashMap::new();
                let mut m = Temp {
                    mode: Mode::COM,
                    st: String::from(""),
                    section_name: String::from(""),
                    section: HashMap::new(),
                    stname: String::new(),
                }; //暂存原始字符串
                for l in br.lines() {
                    match l {
                        Ok(line) => {
                            match &m.mode {
                                Mode::COM => {
                                    //普通
                                    match gettype(&line) {
                                        LineType::STR => {
                                            linecount+=1;
                                        }
                                        LineType::KV => {
                                            let sp: Vec<&str> = line.split(":").collect();
                                            if sp[1].starts_with("\"\"\"") {
                                                //开始
                                                if sp[1].len() >= 6 && sp[1].ends_with("\"\"\"") {
                                                    m.addkv(sp[0].to_string(), sp[1].to_string())
                                                } else {
                                                    m.turn();
                                                    m.setstrname(sp[0].to_string());
                                                    m.setstr(sp[1].to_string()) //开始记录原始字符串
                                                }
                                            } else {
                                                m.addkv(sp[0].to_string(), sp[1].to_string());
                                            }
                                            linecount+=1;
                                        }
                                        LineType::SECTION => {
                                            //此行是section
                                            if !&m.getsname().is_empty() {
                                                //此前存在section
                                                data.insert(m.getsname(), m.getsection());
                                            }
                                            m.clearsection();
                                            m.setsname(line[1..line.len() - 1].to_string());
                                            linecount+=1;
                                        }
                                        LineType::EMPTY => {linecount+=1;}
                                        LineType::UNKNOW => {
                                          linecount+=1;
                                            return Err(format!("{}:文件第{}行格式错误",path.display(),linecount));
                                            
                                        }
                                    }
                                }
                                Mode::STR => {
                                    //原始字符串记录
                                    m.setstr(m.getstr() + "\n" + &line);
                                    if line.ends_with("\"\"\"") {
                                        //println!("原始字符串结束");
                                        m.addkv(m.getstrname(), m.getstr());
                                        m.clearstr();
                                        m.turn();
                                    }
                                    linecount+=1;
                                }
                            }
                        }
                        Err(_) => {
                            println!("Error")
                        }
                    }
                }
                if !m.getsname().is_empty() {
                  data.insert(m.getsname(), m.getsection());
                }
                let mut p = path.clone();
                p.pop();
                return Ok(Ini {path: path.to_path_buf(), data, ppath: p });
            }
            Err(_) => {
                return Err(String::from(format!("打开文件{}失败",path.display())));
            }
        }
    }
    fn code(&self) -> (Ini, Ini, Ini) {
        let mut core_ini = Ini::new();
        let mut conf_ini = Ini::new();
        let mut data_ini = Ini::new();
        for sec in &self.data{
            for (k, v) in sec.1 {
                //过滤不能使用$的键
                match &k[..] {
                    "image"
                    | "name"
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
                      }
                      _=>{
                        let cs = get_bin();//随机节名
                        let ck = get_bin();//随机键名
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
        if !&self["core".to_string()]["copyFrom"].is_empty() {
          let copy_from=self["core".to_string()]["copyFrom"].split(",");
          let mut total_ini = Ini::new();
          for path in copy_from {
            let input:PathBuf;
            let mut tmp = String::from(path);
            while tmp.starts_with(" ") {
              tmp = tmp[1..].to_string();
            }
            if tmp.starts_with("ROOT:") {
              input = root.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
            }else{
              input = self.ppath.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
            }
              match Ini::load_from_file(&input) {
                Ok(ini) => {
                    for (sname,sec) in &ini.data{
                        for (k,v) in sec{
                            total_ini.set_kv(sname.clone(), k.clone(), v.clone());
                        }
                    }
                },
                Err(err) => {return Err(err);},
            }
            if total_ini.data.get("core").unwrap().contains_key("copyFrom") {
                match total_ini.load_copyfrom(root) {
                    Ok(_) => {},
                    Err(err) => {
                        return Err(err);
                    },
                } ;
                total_ini.data.get_mut("core").unwrap().remove("coopyFrom");
            }
          }
          for (sname,sec) in total_ini.data{
            for (k,v) in sec{
              self.set_kv(sname.clone(), k.clone(), v.clone());
            }
          }
          return Ok(());
        }else{
          self["core".to_string()].remove("dont_load");
          return Ok(())
        };
        //self.set_kv("core".to_string(), "dont_load".to_string(), "false".to_string())
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
            data: HashMap::new()
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

fn gettype(line: &String) -> LineType {
  let line = line.trim();
    if line.starts_with("[") {
        LineType::SECTION
    } else if line.contains(":") {
        LineType::KV
    } else if line.is_empty() || line.eq("") || line.replace(" ", "").is_empty() {
        LineType::EMPTY
    } else if line.ends_with("\"\"\"")||line.starts_with("#") {
        LineType::STR
    } else {
        LineType::UNKNOW
    }
}

struct Temp {
    mode: Mode,
    st: String,
    stname: String,
    section_name: String,
    section: HashMap<String, String>,
}

impl Temp {
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


