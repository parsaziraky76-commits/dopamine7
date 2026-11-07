while let Ok(n) = s.read(&mut buf) {
                            if n == 0 || s.write_all(&buf[..n]).is_err() {
                                break;
                            }
                        }
                    }
                });
            }
        });
        port
    }

    fn connect_through(relay: u16) -> TcpStream {
        let mut c = TcpStream::connect(("127.0.0.1", relay)).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        c.write_all(&[5, 1, 0]).unwrap();
        let mut m = [0u8; 2];
        c.read_exact(&mut m).unwrap();
        assert_eq!(m, [5, 0]);
        let name = b"api.cloudflareclient.com";
        let mut req = vec![5, 1, 0, 3, name.len() as u8];
        req.extend_from_slice(name);
        req.extend_from_slice(&443u16.to_be_bytes());
        c.write_all(&req).unwrap();
        c
    }

    /// تورِ کُند (بیش از ده ثانیهٔ هسته نیست، ولی کُند) دیگر شکست حساب نمی‌شود:
    /// جوابِ SOCKS فوری است و داده‌ای که پیش از آمادگیِ تور فرستاده شد، می‌رسد.
    #[test]
    fn a_slow_tor_answer_no_longer_counts_against_the_core() {
        let tor = fake_tor(Duration::from_millis(1500), 0);
        let relay = TorDialRelay::start(tor).unwrap();
        let mut c = connect_through(relay.port());
        let started = Instant::now();
        let mut reply = [0u8; 10];
        c.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0);
        assert!(
            started.elapsed() < Duration::from_millis(1000),
            "reply must not wait for tor"
        );
        c.write_all(b"hello").unwrap();
        let mut echo = [0u8; 5];
        c.read_exact(&mut echo).unwrap();
        assert_eq!(&echo, b"hello");
    }

    /// تورِ ناموفق اتصال را می‌بندد؛ موتور EOF می‌بیند و خودش دوباره می‌کوشد.
    #[test]
    fn a_failed_tor_dial_closes_the_connection() {
        let tor = fake_tor(Duration::from_millis(10), 4);
        let relay = TorDialRelay::start(tor).unwrap();
        let mut c = connect_through(relay.port());
        let mut reply = [0u8; 10];
        c.read_exact(&mut reply).unwrap();
        let mut buf = [0u8; 8];
        let n = c.read(&mut buf).unwrap_or(0);
        assert_eq!(n, 0);
    }

    // >>> AETHER-APP-FIX tor-relay-drops-abandoned-dials
    /// سنجه‌ای که هسته رها کرد، در تور هم رها می‌شود — نه ۵۰ ثانیه بعد.
    /// کانال ready تضمین می‌کند قبل از بستن مشتری، سرور ساختگی درخواست CONNECT را گرفته باشد.
    #[test]
    fn an_abandoned_dial_is_dropped_in_tor_too() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let tor = l.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel::<Duration>();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();

        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut g = [0u8; 3];
            s.read_exact(&mut g).unwrap();
            s.write_all(&[5, 0]).unwrap();

            let mut r = [0u8; 5];
            s.read_exact(&mut r).unwrap();
            let mut rest = vec![0u8; r[4] as usize + 2];
            s.read_exact(&mut rest).unwrap();

            // تورِ کند: هرگز جواب نمی‌دهد؛ فقط منتظر بسته‌شدن اتصال می‌ماند.
            s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
            let began = Instant::now();

            // به تست اعلام می‌کنیم که درخواست CONNECT واقعاً به Tor رسیده است.
            let _ = ready_tx.send(());

            let mut buf = [0u8; 16];
            let _ = s.read(&mut buf);
            let _ = tx.send(began.elapsed());
        });

        let relay = TorDialRelay::start(tor).unwrap();
        let mut c = connect_through(relay.port());
        let mut reply = [0u8; 10];
        c.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0);

        // به‌جای sleep ثابت، تا دریافت کامل CONNECT توسط Tor صبر می‌کنیم.
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("fake tor did not receive CONNECT in time");

        // Data may already be queued (for example a TLS ClientHello). The
        // watcher must buffer it and continue reading until it sees EOF.
        c.write_all(b"queued-client-data").unwrap();

        // حالا که Tor قطعاً درخواست را گرفته، مشتری را رها می‌کنیم.
        drop(c);

        let held = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("tor side closed");
        assert!(
            held < Duration::from_secs(3),
            "abandoned dial held for {held:?}"
        );
    }
    // <<< AETHER-APP-FIX tor-relay-drops-abandoned-dials
}
