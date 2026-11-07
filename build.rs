// exe 파일에 아이콘(assets/icon.ico)을 넣는다. Windows에서만 동작(MinGW의 windres 사용).
fn main() {
    embed_resource::compile("assets/app.rc", embed_resource::NONE).manifest_optional().unwrap();
}
