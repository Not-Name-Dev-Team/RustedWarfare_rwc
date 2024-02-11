
pub(crate) mod ini_unit {
    use std::fs::{copy, File};
    use std::io::{BufRead, BufReader};
    use std::ops::{Index, IndexMut};
    use std::path::PathBuf;
    use colored::Colorize;
    use hashbrown::HashMap;
    use pcre2::bytes::Regex;
    use rand::prelude::ThreadRng;
    use rand::Rng;

    #[derive(Debug)]
    pub(crate) struct Ini {
        pub path:PathBuf,
        pub ppath: PathBuf, //父文件夹
        pub data: HashMap<String, HashMap<String, String>>,
    }

    impl Ini {
        pub(crate) fn load_from_file(path: &PathBuf) -> Result<Ini, String> {
            match File::open(path) {
                Ok(file) => {
                    let mut linecount = 0;
                    let br = BufReader::new(file);
                    let mut data: HashMap<String, HashMap<String, String>> = HashMap::new();
                    let mut tmp = Tmp {
                        mode: Mode::COM,
                        st: String::from(""),
                        section_name: String::from(""),
                        section: HashMap::new(),
                        stname: String::new()
                    }; //暂存原始字符串
                    for l in br.lines() {
                        match l {
                            Ok(line) => {
                                use Mode::*;
                                match &tmp.mode {
                                    COM => {//普通 key:value
                                        use LineType::*;
                                        match tmp.gettype(&line) {
                                            STR => {
                                                linecount+=1;
                                            }
                                            KV => {
                                                if !tmp.getsname().eq("") {
                                                    let (k,v) = line.split_once(":").unwrap();
                                                    if v.starts_with("\"\"\"") {//开始
                                                        if v.len() >= 6 && v.ends_with("\"\"\"") {
                                                            tmp.addkv(k.to_string(), v.to_string())
                                                        } else {
                                                            tmp.turn();
                                                            tmp.setstrname(k.to_string());
                                                            tmp.setstr(v.to_string()) //开始记录原始字符串
                                                        }
                                                    } else {
                                                        tmp.addkv(k.to_string(), v.to_string());
                                                    }
                                                    linecount+=1;
                                                }else {
                                                    return Err(format!("{}{} :第{}行格式错误: {}","[Error]".red(),path.display(),linecount,line));
                                                }
                                            }
                                            SECTION => {
                                                //此行是section
                                                if !&tmp.getsname().is_empty() {
                                                    //此前存在section
                                                    data.insert(tmp.getsname(), tmp.getsection());
                                                }
                                                tmp.clearsection();
                                                tmp.setsname(line[1..line.len() - 1].to_string());
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
                                        tmp.setstr(tmp.getstr() + "\n" + &line);
                                        if line.ends_with("\"\"\"") {
                                            tmp.addkv(tmp.getstrname(), tmp.getstr());
                                            tmp.clearstr();
                                            tmp.turn();
                                        }
                                        linecount+=1;
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    if !tmp.getsname().is_empty() {
                        data.insert(tmp.getsname(), tmp.getsection());
                    }
                    let mut p = path.clone();
                    p.pop();
                    return Ok(Ini {path: path.to_path_buf(), data, ppath: p});
                }
                Err(_) => {
                    return Err(String::from(format!("打开文件 {} 失败",path.display())));
                }
            }
        }
        pub(crate) fn code(&mut self, root:&PathBuf, opath:&PathBuf, random:&mut ThreadRng) -> (Ini, Ini, Ini) {
            let mut core_ini = Ini::new();
            let mut conf_ini = Ini::new();
            let mut data_ini = Ini::new();
            let regex = Regex::new(r#"\$\{.{0,}\}"#).unwrap();
            for sec in &self.data.clone(){
                for (k,v) in sec.1 {
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
                        //其他
                        _=>{
                            let cs = get_bin(random);//随机节名
                            let ck = get_bin(random);//随机键名
                            let mut value = v.clone();

                            //解${}引用
                            //Luke你别老惦记你那ini了
                            if regex.is_match(v.as_bytes()).unwrap(){
                                match regex.captures(v.as_bytes()).unwrap() {
                                    Some(re) => {
                                        let s=std::str::from_utf8(re.get(0).unwrap().as_bytes()).unwrap();
                                        use RefType::*;
                                        match getrf(s) {
                                            SREF => {//跨节引用
                                                let r:Vec<&str> = s.split(".").collect();
                                                let rs= &r[0][2..r[0].len()];
                                                let rk= &r[1][0..r[1].len()-1];
                                                let rv = self.data.get(rs).unwrap().get(rk).unwrap();
                                                value=value.replace(&s,&rv.as_str());
                                            },
                                            REF => {//节内引用
                                                let rk = &s[2..s.len()-1];
                                                let rv=sec.1.get(rk).unwrap();
                                                value=value.replace(&s,&rv.as_str());
                                            },
                                            BDS => {println!("bds:{}",s)},//表达式
                                        }
                                    },
                                    None => todo!(),
                                }
                            }
                            
                            match conf_ini.data.get_mut(sec.0) {
                                Some(conf_sec) => {
                                    conf_sec.insert(k.clone(), String::from("${") + &cs + "." + &ck + "}");
                                }
                                None => {
                                    let mut s: HashMap<String, String> = HashMap::new();
                                    s.insert(k.clone(), String::from("${") + &cs + "." + &ck + "}");
                                    conf_ini.data.insert(sec.0.to_string(), s); //创建节
                                }
                            }
                            match data_ini.data.get_mut(&cs) {
                                Some(data_sec) => {
                                    data_sec.insert(ck.clone(), value.clone());
                                    continue;
                                }
                                None => {
                                    let mut s: HashMap<String, String> = HashMap::new();
                                    s.insert(ck.clone(),value.clone());
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

        pub(crate) fn load_copyfrom(&mut self, root:&PathBuf) ->Result<(), String> {
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
        pub(crate) fn set_kv(&mut self, name: String, k: String, v: String) {
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
        section: HashMap<String, String>
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

    pub(crate) fn get_bin(random:&mut ThreadRng) -> String{
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

    pub(crate) fn get_name(opath:&PathBuf,random:&mut ThreadRng)->String{//获取一个不会重复的文件名
        let a = get_bin(random);
        if opath.join(&a).exists() {
            return get_name(opath,random);
        }else {
            return a;
        }
    }

}