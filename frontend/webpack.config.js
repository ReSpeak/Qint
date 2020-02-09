const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
    entry: './index.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'index.js',
    },
    plugins: [
        new HtmlWebpackPlugin({
            title: "Qint",
            template: "static/index.html"
        }),
        new WasmPackPlugin({
            crateDirectory: path.resolve(__dirname, ".")
        }),
        // Have this example work in Edge which doesn't ship `TextEncoder` or
        // `TextDecoder` at this time.
        new webpack.ProvidePlugin({
            TextDecoder: ['text-encoding', 'TextDecoder'],
            TextEncoder: ['text-encoding', 'TextEncoder']
        })
    ],
    mode: 'development',
    devServer: {
        proxy: [
            {
                context: ['/list', '/audiosend', '/plugins', '/bookmarks', '/messages'],
                target: 'http://localhost:4422'
            },
            {
                context: '/con',
                target: 'http://localhost:4422',
                ws: true
            }
        ],
        contentBase: path.join(__dirname, 'static')
    }
};
