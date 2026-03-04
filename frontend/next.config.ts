import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ['192.168.4.118', '127.0.0.1', 'localhost', 'aikanban.cecep.duckdns.org'],
  async rewrites() {
    return [
      // Proxy REST API calls to the Rust backend
      {
        source: '/api/:path*',
        destination: 'http://localhost:3001/api/:path*',
      },
      // Proxy WebSocket upgrades to the Rust backend
      {
        source: '/ws',
        destination: 'http://localhost:3001/ws',
      },
    ];
  },
};

export default nextConfig;
