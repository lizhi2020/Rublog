use structopt::StructOpt;
use tera::{Tera,Context};
use std::path::{Path,PathBuf};
use std::fs;
use comrak::{markdown_to_html,ComrakOptions};
// todo: use 'content', etc
const default_page_tpl:&str = "<!DOCTYPE html>
<html>
    <head></head>
    <body>{{post_content}}</body>
</html>";
const default_index_tpl:&str = "<!DOCTYPE html>
<html>
    <head></head>
    <body><ul>
        {% for item in dir_list %}
        <li><a href='{{item.path}}'>{{item.name}}</a></li>
        {% endfor %}</ul>
    </body>
</html>";
// --baseUrl --template --index
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt{
    #[structopt(short,long)]
    clear: bool,
    #[structopt(short,long)]
    base_url: Option<String>,
    #[structopt(short,long)]
    template: Option<String>,
    #[structopt(short,long)]
    index: Option<String>
}
#[derive(Debug, serde::Serialize)]
struct Item{
    name: String,
    path: String, // relative path
}
fn main() {
    // println!("Hello, world!");
    let opt = Opt::from_args();
    build(&opt);
}
// render
fn build(opt: &Opt) {
    // init tera
    let mut tera = match Tera::new("templates/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };
    // disable all escape
    tera.autoescape_on(vec![]);
    let names: Vec<_> = tera.get_template_names().collect();
    if !names.contains(&"default-page.html"){
        tera.add_raw_template("default-page.html", default_page_tpl).unwrap();
    }
    let names: Vec<_> = tera.get_template_names().collect(); // ugly
    if !names.contains(&"default-index.html"){
        tera.add_raw_template("default-index.html", default_index_tpl).unwrap();
    }
    // check clear
    let path = Path::new("public");
    if opt.clear{
        if path.exists(){
            fs::remove_dir_all(&path).unwrap_or_else(|e|{
                println!("{}{:?}",e,&path);
            });
        }
    }
    // create dir
    if !path.exists(){
        fs::create_dir(&path).unwrap();
    }
    // render md only
    render_dir(&tera, Path::new("content"), Path::new("public"));
}
// recurisive render a dir
fn render_dir(tera:&Tera,src_dir:&Path,dst_dir:&Path){
    for entry in fs::read_dir(src_dir).unwrap(){
        let entry=entry.unwrap();
        let path=entry.path();
        if !dst_dir.exists(){
            fs::create_dir_all(&dst_dir).unwrap();
        }
        let mut dst_dir = PathBuf::from(dst_dir);
        dst_dir.push(path.file_name().unwrap());
        //let mut dst=PathBuf::from("public");
        //dst.push(path.file_name().unwrap());
        if path.is_dir(){
            render_dir(tera, &path, &dst_dir)
        }
        else if path.extension().unwrap() == "md"{
            dst_dir.set_extension("html");
            // render and write. opt
            render(&tera,&path,&dst_dir).unwrap();
        }
    }
}
// render a single file
fn render(tera:&Tera,src:&PathBuf,dst:&PathBuf)->std::io::Result<()>{
    let src_content=fs::read_to_string(src)?;
    let post_context=markdown_to_html(&src_content, &ComrakOptions::default());

    let mut context=Context::new();
    
    let mut post_list = Vec::new();
    for entry in fs::read_dir(src.parent().unwrap()).unwrap(){
        let path=entry?.path();
        // let file_name=path.file_name().unwrap().to_str().unwrap().to_string();
        let file_name=path.file_stem().unwrap().to_os_string().into_string().unwrap();
        let file_path:String;
        if path.is_dir(){
            file_path = file_name.clone();
        }
        else if "md" == path.extension().unwrap(){
            let mut tmp = file_name.clone();
            tmp.push_str(".html");
            file_path = tmp;
        }
        else{
            continue;
        }
    /*
        let pos=file_name.rfind('.').unwrap();
        let (file_name,_)=file_name.split_at(pos);
        let file_name=file_name.to_string();

        if file_name==String::from("index"){
            continue;
        }
        
        let last_mod= fs::metadata(path)?.modified()?.elapsed().unwrap().as_secs();
        let mut url=String::new();
        url.push_str(file_name.as_str());
        url.push_str(".html");
    */
        // todo: add time, size, etc..
        post_list.push(Item{name:file_name,path:file_path});
    };

    // post_list.sort_by_key(|k| k.time);
    context.insert("dir_list", &post_list);
    

    context.insert("post_content", &post_context);
    let template_type:&str;
    // if not front-matter !
    println!("{}",dst.file_name().unwrap().to_str().unwrap());
    if src.file_name().unwrap().to_str().unwrap() == "index.md"{
        template_type="default-index.html";
    }
    else{
        template_type="default-page.html";
    }
    let content=tera.render(template_type, &context).unwrap();
    // if parent dir doesn't exist?
    fs::write(dst, content).unwrap();
    Ok(())
}