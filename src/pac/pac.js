var hosts = __HOSTS__;
var proxy = "SOCKS5 127.0.0.1:1080; SOCKS 127.0.0.1:1080; DIRECT;";
var direct = "DIRECT;";

function FindProxyForURL(_url, host) {
  if (hosts.has(host)) {
    return proxy;
  }
  return direct;
}
