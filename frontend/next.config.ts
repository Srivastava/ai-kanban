import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: 'export',
  trailingSlash: true,
  compress: true,
  productionBrowserSourceMaps: false,
};

export default nextConfig;
