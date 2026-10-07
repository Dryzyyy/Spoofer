//! Tests headless de la logique portée depuis Python (aucun changement réseau).
#[cfg(test)]
mod tests {
    use crate::net;
    use crate::proxy;
    use crate::settings;
    use crate::tun;

    #[test]
    fn mac_utils() {
        assert_eq!(net::clean_mac("02:11:22:33:44:55"), "021122334455");
        assert_eq!(net::clean_mac("02-11-22-33-44-55"), "021122334455");
        assert_eq!(net::dash_mac("021122334455"), "02-11-22-33-44-55");
        assert!(net::valid_mac_nosep("02:11:22:33:44:55"));
        assert!(!net::valid_mac_nosep("02:11:22"));
        let m = net::random_mac_colon();
        assert!(net::valid_mac_nosep(&m), "random invalide: {m}");
        // locally administered : 2e bit du 1er octet = 1
        let first: u8 = u8::from_str_radix(&net::clean_mac(&m)[0..2], 16).unwrap();
        assert_eq!(first & 0x02, 0x02);
    }

    #[test]
    fn bypass_parse() {
        assert_eq!(net::parse_bypass("Discord.exe"), vec!["Discord.exe"]);
        assert_eq!(
            net::parse_bypass("Discord.exe, Spotify.exe;Foo.exe\nBar.exe"),
            vec!["Discord.exe", "Spotify.exe", "Foo.exe", "Bar.exe"]
        );
        assert!(net::parse_bypass("").is_empty());
    }

    #[test]
    fn proxy_strings() {
        assert_eq!(
            proxy::proxy_server_string("SOCKS5 (Tor / autre)", "127.0.0.1", "9050"),
            Some("socks=127.0.0.1:9050".into())
        );
        assert_eq!(
            proxy::proxy_server_string("HTTP", "51.15.1.2", "8080"),
            Some("51.15.1.2:8080".into())
        );
        assert_eq!(proxy::proxy_server_string("HTTP", "", "8080"), None);
    }

    #[test]
    fn wg_parse() {
        let dir = std::env::temp_dir();
        let p = dir.join("ghostnet-test.conf");
        std::fs::write(
            &p,
            "[Interface]\nPrivateKey = AAAA\nAddress = 10.64.0.2/32\nDNS = 10.64.0.1\n\n[Peer]\nPublicKey = BBBB\nEndpoint = 185.65.134.66:51820\nAllowedIPs = 0.0.0.0/0\n",
        )
        .unwrap();
        let wg = tun::parse_wg_conf(p.to_str().unwrap()).unwrap();
        assert_eq!(wg.endpoint_host, "185.65.134.66");
        assert_eq!(wg.endpoint_port, 51820);
        assert_eq!(wg.addresses, vec!["10.64.0.2/32"]);
        let cfg = tun::build_wg_config(&wg, &["Discord.exe".to_string()]);
        assert_eq!(cfg["route"]["final"], "proxy");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn socks_config_shape() {
        let cfg = tun::build_socks_config("127.0.0.1", 9050, "", "", &[]);
        assert_eq!(cfg["inbounds"][0]["interface_name"], "GhostTUN");
        assert_eq!(cfg["outbounds"][0]["server_port"], 9050);
        assert_eq!(cfg["route"]["final"], "proxy");
        // sing-box exige version en CHAÎNE ("5"), pas en nombre (bug réel constaté :
        // "cannot unmarshal number into Go value of type string").
        assert_eq!(cfg["outbounds"][0]["version"], serde_json::json!("5"));
        // la config générée doit passer le vrai `sing-box check`
        let tmp = std::env::temp_dir().join("ghostnet-test-sing.json");
        std::fs::write(&tmp, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();
        let exe = crate::paths::spoofer_dir().unwrap().join("bin").join("sing-box.exe");
        assert!(exe.exists(), "sing-box manquant pour le test");
        let out = std::process::Command::new(&exe)
            .args(["check", "-c"])
            .arg(&tmp)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "sing-box check refusé: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn settings_defaults() {
        let s = settings::load_settings();
        // doit au minimum contenir les clés par défaut même sans fichier
        for k in ["ptype", "phost", "pport", "ip_method", "mac_iface", "bypass_apps"] {
            assert!(s.contains_key(k), "clé manquante: {k}");
        }
    }

    // --- lecture seule sur la machine (pas de modification) ---

    #[test]
    fn adapters_present() {
        let ads = net::get_adapters();
        assert!(!ads.is_empty(), "aucun adaptateur détecté");
    }

    #[test]
    fn proxy_read_no_panic() {
        let _ = proxy::get_system_proxy();
    }

    #[test]
    fn tun_tor_detected() {
        // bin/ existe dans D:\Spoofer (tests lancés depuis gui/ -> spoofer_dir remonte)
        assert!(tun::is_installed(), "sing-box non détecté");
        assert!(crate::tor::is_installed(), "tor non détecté");
    }

    #[test]
    fn native_parity() {
        let native = net::get_adapters_native();
        assert!(native.is_some(), "GetAdaptersAddresses a échoué");
        let native = native.unwrap();
        assert!(!native.is_empty());
        let eth = native.iter().find(|a| a.name == "Ethernet");
        assert!(eth.is_some(), "Ethernet absent du natif");
        let eth = eth.unwrap();
        assert!(eth.mac.len() == 17, "MAC inattendue: {}", eth.mac);
        assert!(eth.guid.starts_with('{'), "GUID inattendu: {}", eth.guid);
        assert!(eth.ifindex.unwrap_or(0) > 0);
        println!("NATIVE eth: mac={} ip={} status={}", eth.mac, eth.ip, eth.status);
        for a in &native {
            println!("NATIVE {} ip={} eff={} status={}", a.name, a.ip, a.mac, a.status);
        }
    }

    #[test]
    fn timing_probe() {
        use std::time::Instant;
        let t = Instant::now();
        let ads = net::get_adapters();
        println!("PROBE adapters: {}ms n={}", t.elapsed().as_millis(), ads.len());
        let t = Instant::now();
        let _ = proxy::get_system_proxy();
        println!("PROBE proxy: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let mut n_spoof = 0;
        for a in &ads {
            if crate::mac::get_spoofed_mac(a).is_some() {
                n_spoof += 1;
            }
        }
        println!("PROBE spoof-check x{}: {}ms (spoofed={n_spoof})", ads.len(), t.elapsed().as_millis());
        let t = Instant::now();
        let _ = crate::tor::status();
        println!("PROBE tor.status: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let _ = crate::tor::is_running();
        println!("PROBE tor.is_running: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let _ = crate::tor::socks_ready();
        println!("PROBE tor.socks_ready: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let _ = crate::tor::bootstrap_pct();
        println!("PROBE tor.bootstrap: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let _ = crate::elevation::is_admin();
        println!("PROBE is_admin: {}ms", t.elapsed().as_millis());
        let t = Instant::now();
        let ip = net::get_public_ip();
        println!("PROBE public_ip: {}ms -> {ip}", t.elapsed().as_millis());
    }

    #[test]
    fn public_ip_format() {
        let ip = net::get_public_ip();
        // soit une IPv4, soit ERR explicite
        assert!(ip.contains('.') || ip.starts_with("ERR"), "IP inattendue: {ip}");
    }
}
