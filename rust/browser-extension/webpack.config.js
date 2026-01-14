const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: {
    background: './src/background/index.ts',
    popup: './src/popup/popup.ts',
    options: './src/options/options.ts',
    content: './src/content/content.ts',
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].js',
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js'],
  },
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'manifest.json', to: '.' },
        { from: 'popup.html', to: '.' },
        { from: 'options.html', to: '.' },
        { from: 'static', to: 'static' },
        { from: 'wasm', to: 'wasm' },
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
};
