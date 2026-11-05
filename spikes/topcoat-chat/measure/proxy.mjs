// A TCP proxy the reconnect test can cut: cut() resets every connection and refuses new ones until restore().
import net from 'node:net';
export function proxy(listenPort, target) {
  const socks = new Set(); let down = false;
  const server = net.createServer(c => {
    if (down) return c.resetAndDestroy();
    const u = net.connect(target, '127.0.0.1');
    socks.add(c); socks.add(u);
    c.pipe(u); u.pipe(c);
    const end = () => { socks.delete(c); socks.delete(u); c.destroy(); u.destroy(); };
    c.on('error', end); u.on('error', end); c.on('close', end); u.on('close', end);
  });
  return new Promise(r => server.listen(listenPort, '127.0.0.1', () => r({
    cut() { down = true; for (const s of socks) s.resetAndDestroy(); socks.clear(); },
    restore() { down = false; },
    close() { server.close(); for (const s of socks) s.destroy(); },
  })));
}
