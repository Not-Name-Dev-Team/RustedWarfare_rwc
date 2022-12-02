use clap::{App, load_yaml};
use ini::{Ini, Properties};
use colored::*;
use rand::rngs::ThreadRng;
use std::ffi::OsStr;
use std::io::{Write, self};
use std::process::exit;
use std::path::{Path,PathBuf};
use std::fs::{create_dir_all, OpenOptions};
use std::thread;
use std::time::Duration;
use rand::Rng;
use indicatif::{ProgressBar,ProgressStyle};

#[derive(Debug)]
struct Unit{
  name:String,
  data:Vec<(
    String,//section
    Vec<(
      String,//key
      String //value
    )>
  )>,
  path:PathBuf
}

fn main() ->() {

  let bar = ProgressBar::new_spinner();
  bar.enable_steady_tick(Duration::from_millis(1200));
  
  match ProgressStyle::default_spinner().tick_strings(&[".","..","...",]).template("{prefix:.bold.dim} {spinner:.white} {msg}"){
    Ok(sty)=> {bar.set_style(sty);}
    Err(err)=>{println!("{}{}","[Error]".red(),err)}
  }

  let bar_clone = bar.clone();

  thread::spawn(move || {
    loop {
        bar_clone.inc(1);
    }
    
  });

  let yml = load_yaml!("yaml.yml");//使用load_yaml宏读取yaml.yml内的内容
  let matches = App::from_yaml(yml).get_matches();

  let error_text = "[Error]".red();

  let mut units:Vec<Unit> = vec![];

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
      load_ini(path,&mut units,&root,&bar);
    }else if path.is_dir() {
      load_dir(path, &mut units,root,&bar);
    }else {
      bar.println(format!("{}输入文件不存在",error_text));
      return;
    }
    bar.set_prefix("Writing");
    for unit in &units{
      output(&unit,&opath,&bar);
    }
    bar.println(format!("{}所有文件输出完成","[Log]".blue()));
    bar.finish_and_clear();
  }

  //检测ini
  if units.is_empty() {
    bar.println(format!("{}缺少有效ini文件输入",error_text));
    bar.finish_and_clear();
    exit(0);
  }
}

//加载文件夹内ini
fn load_dir(f:PathBuf,units:&mut Vec<Unit>,root:PathBuf,bar:&ProgressBar){
  bar.set_prefix("Reading ");
  for entry in walkdir::WalkDir::new(f){
    if entry.as_ref().unwrap().path().extension().eq(&Some(OsStr::new("ini"))) {
      bar.set_message(entry.as_ref().unwrap().path().to_str().unwrap().to_string());
      load_ini(PathBuf::from(entry.unwrap().path()), units,&root,bar);
    }
  }
}

//加载ini
fn load_ini(f:PathBuf,units:&mut Vec<Unit>,root:&PathBuf,bar:&ProgressBar) {
  let error_text = "[Error]".red();
  let warn_text = "[Warn]".yellow();
  let mut path=f.clone();
  match Ini::load_from_file(f) {//读取ini文件
    Ok(ini) => {//读取成功
      
      let mut unit:Unit;
      let mut data:Vec<(String,Vec<(String,String)>)>=vec![];
      
      let sec = &ini.section(Some("core")); //读取[core]
      let core:&Properties;
      if sec.is_none(){
        bar.println(format!("{}{}内不存在[core],跳过该文件",warn_text,path.display()));
        return;
      }else{
        core = sec.unwrap();
      }
      if core.get("name").is_none() {
          bar.println(format!("{}{}内不存在name,跳过该文件",warn_text,path.display()));
          return;
      }
      //遍历ini内所有节
      for (section,prop) in ini.iter(){
        //获取节名
        let section_name =  section.as_ref().unwrap();
        //遍历节内数据
        let mut tmp:Vec<(String,String)> = vec![];
        for (k,v) in prop.iter(){
          tmp.push((String::from(k),String::from(v)));
        }
        data.push((String::from(*section_name),tmp));
      }
      path.pop();
      unit=Unit{name:String::from(core.get("name").unwrap()),data:data,path};
      
      //追加copyFrom数据
      if !core.get("copyFrom").is_none() {
        let copy = core.get("copyFrom").unwrap();
        for i in copy.split(&[','][..]) {
          let input:PathBuf;
          let mut tmp = String::from(i);
          while tmp.starts_with(" ") {
            tmp = tmp[1..].to_string();
          }
          if tmp.starts_with("ROOT:") {
            input = root.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
          }else{
            input = unit.path.join(tmp.replace("\n", "\\n").replace("ROOT:/", "").replace("ROOT:", ""));
          }
          
          match unit.load_temp(&input,bar){
            1=>{}
            _=>{
              return;
            }
          };
        }
      }
      units.push(unit);
    },
    Err(err) => {
      println!("{}{} File:{}",error_text,err,path.display());
      exit(1);
    },
  };
}

//Unit追加temp内数据
impl Unit {
  fn load_temp(&mut self,f:&PathBuf,bar:&ProgressBar) -> u8 {
    match Ini::load_from_file(f) {//读取ini文件
      Ok(ini) => {//读取成功
        //遍历节
        for (section,prop) in ini.iter(){
          //获取节名
          let section_name =  section.as_ref().unwrap();
          let unit_section = self.get_section(String::from(*section_name));
          //遍历节内数据
          for (k,v) in prop.iter(){
            unit_section.1.push((String::from(k),String::from(v)));
          }
        }
        return 1;
      },
      Err(err) => {
        bar.println(format!("{}{}加载{}至{}出错,可能文件不符合格式","[Error]".red(),err,f.display(),&self.path.join(&self.name).display()));
        return 0;
      }
    }
  }

  fn get_section(&mut self,name:String)-> &mut (String, Vec<(String, String)>) {
    let re:&mut (String, Vec<(String, String)>);
    if self.has_section(&name){
      for i in self.data.iter_mut(){
        if i.0.eq(&name) {
            re=i;
            return re;
        }
      };
    }else{
      self.data.push((name.clone(),vec![]));
      for i in self.data.iter_mut(){
        if i.0.eq(&name) {
            re=i;
            return re;
        }
      };
    }
    
    println!("{}Unknow","[Error]".red());
    exit(0);
  }
  fn has_section(&mut self,name:&String) -> bool {
    let mut re=false;
    for i in self.data.iter(){
      if i.0.eq(name) {
        re=true;
      }
    }
    return re;
  }
}

fn output(unit:&Unit,opath:&Path,bar:&ProgressBar){
  let name = &unit.name;
  let core=opath.join(name.clone()+"_core.ini");
  let data=opath.join(name.clone()+"_data");
  let conf=opath.join(name.clone()+"_conf");
  let mut random: ThreadRng = rand::thread_rng();
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

  let mut core_ini = ini::Ini::new();
  let mut conf_ini = ini::Ini::new();
  let mut data_ini = ini::Ini::new();

  for section in &unit.data{
        for (k,v) in section.1.clone(){
          //过滤不能使用$的键
          match &k[..] {
            "image" | "name" | "copyFrom" | "altNames" | "class" | "overrideAndReplace" | "onNewMapSpawn" | "canOnlyBeAttackedByUnitsWithTags" | "tags" | "similarResourcesHaveTag" | "displayText" | "displayDescription" |
            "displayLocaleKey" | "transportUnitsRequireTag" | "canReclaimResourcesOnlyWithTags" | "unitsSpawnedOnDeath" | "soundOnDeath" | "isLockedMessage" | "isLockedAltMessage" | "isLockedAlt2Message" | "teamColoringMode" | 
            "drawLayer" | "attackMovement" | "interceptProjectiles_withTags" | "shoot_sound" | "friendlyFire" | "movementType" | "upgradedFrom" | "onlyUseAsHarvester_ifBaseHasUnitTagged" | "priority" | "stripIndex" | "onActions" | 
            "text" | "textPostFix" | "description" | "displayType" | "showMessageToAllEnemyPlayers" | "showQuickWarLogToPlayer" | "showQuickWarLogToAllPlayers" | "anyRuleInGroup" | "cannotPlaceMessage" | "displayName" | "displayNameShort" |
            "autoTriggerOnEvent" | "fireTurretXAtGround_onlyOverPassableTileOf" | "deleteNumUnitsFromTransport_onlyWithTags" | "addWaypoint_target_randomUnit_team" | "attachments_onlyOnSlots" | "showMessageToPlayer" | "showMessageToAllPlayers" | 
            ""=>{
              core_ini.set_to(Some(String::from(section.0.as_str())), k.clone(), v.clone());
            }
            _ => {
              let cs=to_bin(random.gen::<u32>());
              let ck=to_bin(random.gen::<u32>());
              conf_ini.set_to(Some(String::from(section.0.as_str())), k.clone(), String::from("${") + &cs + "." + &ck + "}");
              data_ini.set_to(Some(cs), ck, v);
            }
          }
        }
  }

  core_ini.set_to(Some("core"), String::from("copyFrom"), String::from("") + conf.file_name().unwrap().to_str().unwrap() + "," + data.file_name().unwrap().to_str().unwrap());
  core_ini.delete_from(Some("core"), "dont_load");
    bar.set_message(format!("{} -> {}",name,core.display()));
    match write_to(&core_ini,&mut core_file.unwrap()){
    Ok(())=>{},
      Err(err)=>{println!("{}{}","[Error]".red(),err)}
    };
    bar.set_message(format!("{} -> {}",name,conf.display()));
    match write_to(&conf_ini,&mut conf_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{println!("{}{}","[Error]".red(),err)}
    };
    bar.set_message(format!("{} -> {}",name,data.display()));
    match write_to(&data_ini,&mut data_file.unwrap()){
      Ok(())=>{},
      Err(err)=>{println!("{}{}","[Error]".red(),err)}
    };
}

//输出ini到文件
fn write_to<W: Write>(ini: &ini::Ini, writer: &mut W) -> io::Result<()> {
    for (section,prop) in ini.iter(){
      //获取节名
      let section_name =  section.as_ref().unwrap();
      //遍历节内数据
      writeln!(writer, "[{}]",section_name)?;
      for (k,v) in prop.iter(){
        writeln!(writer, "{}:{}", k, v)?;
      }
    }
    Ok(())
}

fn to_bin(i:u32) -> String{
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