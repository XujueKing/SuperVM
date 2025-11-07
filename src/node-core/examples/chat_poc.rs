use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic as Topic, MessageAuthenticity},
    identify, mdns, ping, relay, dcutr,
    swarm::{NetworkBehaviour, SwarmEvent}, 
    Multiaddr,
};
use std::time::Duration;
use tokio::{select, io::{AsyncBufReadExt, BufReader}};

#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    relay_client: relay::client::Behaviour,
    dcutr: dcutr::Behaviour,
}

#[derive(Parser, Debug)]
#[command(name = "chat_poc", about = "SuperVM Chat PoC (gossipsub + mDNS + QUIC/DCUtR/Relay)")]
struct Args {
    /// Optional multiaddr to dial, e.g. /ip4/192.168.1.10/tcp/4001
    #[arg(long)]
    dial: Option<String>,

    /// Use QUIC instead of TCP
    #[arg(long)]
    quic: bool,

    /// Optional relay address for NAT traversal, e.g. /ip4/relay.example.com/tcp/4001/p2p/12D3K...
    #[arg(long)]
    relay: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1) 创建 Swarm (使用 libp2p 0.53 的 builder API)
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            let local_peer_id = key.public().to_peer_id();
            println!("Local peer id: {local_peer_id}");
            
            // gossipsub 配置
            let gs_config = gossipsub::ConfigBuilder::default()
                .validation_mode(gossipsub::ValidationMode::Strict)
                .heartbeat_interval(Duration::from_secs(1))
                .max_transmit_size(1 << 20)
                .build()
                .expect("valid gossipsub config");
            
            let gossipsub = gossipsub::Behaviour::new(
                MessageAuthenticity::Signed(key.clone()), 
                gs_config
            ).expect("gossipsub init");
            
            // mdns 本地发现
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(), 
                local_peer_id
            ).expect("mdns init");
            
            // identify & ping
            let identify = identify::Behaviour::new(
                identify::Config::new("chat-poc/1.0".into(), key.public())
            );
            let ping = ping::Behaviour::default();
            
            // dcutr
            let dcutr = dcutr::Behaviour::new(local_peer_id);
            
            Ok(ChatBehaviour { 
                gossipsub, 
                mdns, 
                identify, 
                ping, 
                relay_client, 
                dcutr 
            })
        })?
        .build();

    // 2) 监听 0.0.0.0:0 (随机端口)
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?)?;
    if args.quic {
        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>()?)?;
    }

    // 10) 订阅聊天主题
    let topic = Topic::new("supervm-chat");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    println!("Chat PoC started. Type and press Enter to publish.");
    if args.quic {
        println!("QUIC mode enabled.");
    }
    println!();

    // 从 stdin 读输入
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();

    // 如果提供了 --relay，先连接中继
    if let Some(relay_addr) = args.relay.as_deref() {
        if let Ok(ma) = relay_addr.parse::<Multiaddr>() {
            println!("Connecting to relay {ma} ...");
            let _ = swarm.dial(ma.clone());
            // 监听中继转发地址（待连接成功后由事件处理）
        } else {
            eprintln!("Invalid relay multiaddr: {relay_addr}");
        }
    }

    // 如果提供了 --dial，主动拨号
    if let Some(addr) = args.dial.as_deref() {
        if let Ok(ma) = addr.parse::<Multiaddr>() {
            println!("Dialing {ma} ...");
            let _ = swarm.dial(ma);
        } else {
            eprintln!("Invalid multiaddr: {addr}");
        }
    }

    loop {
        select! {
            line = stdin.next_line() => {
                if let Ok(Some(text)) = line {
                    let payload = text.into_bytes();
                    let _ = swarm.behaviour_mut().gossipsub.publish(topic.clone(), payload);
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {address}");
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(ev)) => {
                        match ev {
                            mdns::Event::Discovered(list) => {
                                for (_peer, addr) in list {
                                    // 自动拨号以加快连通
                                    let _ = swarm.dial(addr.clone());
                                }
                            }
                            mdns::Event::Expired(_) => {}
                        }
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(ev)) => {
                        if let gossipsub::Event::Message{ propagation_source, message, .. } = ev {
                            if let Ok(text) = String::from_utf8(message.data) {
                                println!("<{}> {}", propagation_source, text);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::RelayClient(ev)) => {
                        println!("RelayClient event: {:?}", ev);
                        // 当成功连接到中继后，监听中继转发地址
                        if let relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } = ev {
                            println!("Relay reservation accepted by {relay_peer_id}");
                        }
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Dcutr(ev)) => {
                        println!("Dcutr event: {:?}", ev);
                        // DCUtR 尝试直接连接升级
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Identify(ev)) => {
                        // Identify 协议事件（可选打印）
                        if let identify::Event::Received { peer_id, info } = ev {
                            println!("Identified {peer_id}: protocols={:?}", info.protocols);
                        }
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Ping(ev)) => {
                        // Ping 协议事件（可选打印）
                        if let ping::Event { peer, result: Ok(_), .. } = ev {
                            println!("Ping to {peer} succeeded");
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
