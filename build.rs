fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "core-interfaces/proto";

    // Derleme script'ine, submodule klasöründeki .proto dosyaları değiştiğinde
    // yeniden çalışmasını söyle. Bu, önbellekleme için önemlidir.
    println!("cargo:rerun-if-changed={}", proto_root);

    let proto_files = &[
        format!("{}/sentiric/media/v1/media.proto", proto_root),
        format!("{}/sentiric/user/v1/user.proto", proto_root),
        format!("{}/sentiric/dialplan/v1/dialplan.proto", proto_root),
    ];

    tonic_build::configure()
        // Bu servis, diğer servislere sadece istek atacağı için bir sunucu
        // implementasyonuna ihtiyacı yok. Bu, derleme boyutunu küçültür.
        .build_server(false) 
        .compile(
            proto_files, // Derlenecek .proto dosyalarının listesi
            &[proto_root], // .proto dosyalarının import edilebileceği ana klasör
        )?;

    Ok(())
}