use structopt::StructOpt;
use tera::{Tera,Context};
use std::path::{Path,PathBuf};
use std::fs;
use std::io;
use comrak::{markdown_to_html,ComrakOptions};
// todo: use 'content', etc
const DEFAULT_PAGE_TPL_NAME:&str = "default-page.html";
const DEFAULT_INDEX_TPL_NAME:&str = "default-index.html";
const DEFAULT_PAGE_TPL:&str = "<!DOCTYPE html>
<html>
    <head></head>
    <body>{{content}}</body>
</html>";
const DEFAULT_INDEX_TPL:&str = "<!DOCTYPE html>
<html>
    <head></head>
    <body><ul>
        {% for item in dir_list %}
        <li><a href='{{item.path}}'>{{item.name}}</a></li>
        {% endfor %}</ul>
    </body>
</html>";
// --baseUrl --template --index --theme
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt{
    #[structopt(short,long)]
    clear: bool,
    #[structopt(short,long)]
    base_url: Option<String>,
    #[structopt(long)]
    template: Option<String>,
    #[structopt(short,long)]
    index: Option<String>,
    #[structopt(long)]
    theme: Option<String>,
}
#[derive(Debug, serde::Serialize)]
struct Item{
    name: String,
    path: String, // relative path
}

#[derive(serde::Deserialize)]
struct FrontMatter{
    template: Option<String>
}

#[derive(serde::Serialize)]
struct Post{
    url: String,
    title: String,
    content: String,
    template: Option<String>,
}
fn main() {
    // println!("Hello, world!");
    let opt = Opt::from_args();
    build(&opt);
}
// load template
fn init_tera(dir: &str)->tera::Tera{
    let mut tera = match Tera::new(&dir) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };
    // disable all escape
    tera.autoescape_on(vec![]);
    let names: Vec<_> = tera.get_template_names().collect();
    if !names.contains(&DEFAULT_PAGE_TPL_NAME){
        tera.add_raw_template(DEFAULT_PAGE_TPL_NAME, DEFAULT_PAGE_TPL).unwrap();
    }
    let names: Vec<_> = tera.get_template_names().collect(); // ugly
    if !names.contains(&DEFAULT_INDEX_TPL_NAME){
        tera.add_raw_template(DEFAULT_INDEX_TPL_NAME, DEFAULT_INDEX_TPL).unwrap();
    };
    tera
}
// copy css files
fn copy_files(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if !ty.is_dir() {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            // fs::copy("themes/nextr/css/next.css", "public/css/next.css")?;
        };
    }
    Ok(())
}
// render
fn build(opt: &Opt) {
    // check clear
    let path = Path::new("public");
    if opt.clear{
        if path.exists(){
            fs::remove_dir_all(&path).unwrap_or_else(|e|{
                println!("{}{:?}",e,&path);
            });
        }
    }
    // check theme validate            why &opt.theme
    let mut tpl_dir = if let Some(theme) = &opt.theme{
        let mut tpl_dir = String::from("themes/");
        tpl_dir.push_str(&theme);
        tpl_dir.push('/');
        tpl_dir
    } else {
        String::from("")
    };
    let mut css_dir = tpl_dir.clone();
    css_dir.push_str("css");
    copy_files(Path::new(&css_dir), Path::new("public/css")).unwrap();
    // copy css files
    tpl_dir.push_str("template/*.html");
    // init tera
    let tera = init_tera(&tpl_dir);
    
    // create dir
    if !path.exists(){
        fs::create_dir(&path).unwrap();
    }
    // render md only
    render_dir(&tera, Path::new("content"), Path::new("public"), &opt);
}
// recurisive render a dir
fn render_dir(tera:&Tera,src_dir:&Path,dst_dir:&Path, opt: &Opt){

    let mut index = false;
    let mut posts = Vec::new();
    for entry in fs::read_dir(src_dir).unwrap(){
        let entry=entry.unwrap();
        let path=entry.path();
        if !dst_dir.exists(){
            fs::create_dir_all(&dst_dir).unwrap();
        }
        let mut dst_dir = PathBuf::from(dst_dir);
        let entry_name = path.file_name().unwrap();
        if entry_name == "index.md" {
            index = true;
            continue;
        }
        dst_dir.push(path.file_name().unwrap());
        if path.is_dir(){
            render_dir(tera, &path, &dst_dir, &opt)
        }
        else if path.extension().unwrap() == "md"{
            dst_dir.set_extension("html");
            // render and write. opt
            let post = meta_from_file(&path).unwrap();
            let template = choose_template(&path, &opt, &post);
            render(&tera, &dst_dir, &template, &Context::from_serialize(&post).unwrap()).unwrap();
            posts.push(post);
        }
    }

    if index {
        let from = src_dir.to_path_buf().join("index.md");
        let to = dst_dir.to_path_buf().join("index.html");
        let post = meta_from_file(&from).unwrap();
        let template = choose_template(&from, &opt, &post);
        let mut ctx = Context::from_serialize(&post).unwrap();
        ctx.insert("posts",&posts);
        render(&tera, &to, &template, &ctx).unwrap();
    }
}
fn meta_from_file(src:&Path) -> io::Result<Box<Post>>{
    let src_content=fs::read_to_string(src)?;
    // todo opt
    let (p1, p2) = if src_content.starts_with("+++\r\n"){
        if let Some(t) = &src_content[3..].find("+++"){
            let (p1, p2) = src_content.split_at(*t+3);
            let p1 = p1.split_at(5).1;
            let p2 = if p2.len() > 5 { p2.split_at(5).1 } else {""};
            (Some(p1), p2)
        }else{
            (None,src_content.as_str())
        }
    }else{
        (None,src_content.as_str())
    };
    // should not panic here
    let front_matter : FrontMatter = if let Some(t) = p1 {
        toml::from_str(t).unwrap() // "".prase::<>().unwrap()
    } else {
        toml::from_str("").unwrap()
    };
    // &p2?
    let content = markdown_to_html(p2, &ComrakOptions::default());
    let title = String::from(src.file_stem().unwrap().to_str().unwrap());
    let url = String::from(src.strip_prefix("content").unwrap().to_str().unwrap());
    let template = front_matter.template;
    Ok(Box::new(Post{title,content,url,template}))
}

fn render(tera:&Tera,dst:&Path,template: &String,ctx: &tera::Context) -> std::io::Result<()>{
    let content = tera.render(template, ctx).unwrap(); // &ctx ?
    // if parent dir doesn't exist?
    fs::write(dst, &content)?;
    Ok(())
}
fn choose_template(src: &Path, opt: &Opt, post: &Post) -> String{
    if let Some(tpl) = &post.template {
        return tpl.clone();
    }    

    let mut tpl1=String::from(DEFAULT_INDEX_TPL_NAME);
    let mut tpl2=String::from(DEFAULT_PAGE_TPL_NAME);
    if let Some(tpl) = &opt.index {
        tpl1 = tpl.clone();
    }
    if let Some(tpl) = &opt.template {
        tpl2 = tpl.clone();
    }
    if src.file_name().unwrap().to_str().unwrap() == "index.md"{
        tpl1
    }
    else{
        tpl2
    }
}