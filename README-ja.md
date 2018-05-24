# ALVR - Air Light VR

ALVRはPCVRの画面をGear VRやOculus Goに転送して遊ぶためのソフトです。SteamVRのゲームをスタンドアロン型のヘッドセットでプレイすることができます。

## 特徴
ALVRはPCVRの画面をエンコードしWi-Fi経由でヘッドセットに転送します。似たソフトとしてRiftcatやTrinus VRがありますが、それらよりもGear VR向けに特化しているのが特徴です。Wi-Fi経由でもGear VRに搭載されたAsynchronous Timewarpを利用してスムーズなヘッドトラッキングを実現できます。

## 動作環境
以下の動作環境が必要です。
- Gear VR か Oculus Go
 - 現状、Gear VR + Galaxy S8でしか試していません。S6やS7だとスペック的に厳しいかもしれません。
 - 試した方がいたらフィードバックお願いします！
- NVENCが使えるNVIDIA GPUを搭載したハイエンドPC
 - 現在、Windows 10のみサポートしています。
- 802.11n/ac Wi-Fi

## インストール方法
- ALVR serverのインストールする方法
 - release pageからzipをダウンロード
 - 任意のフォルダに展開
 - driver\_install.batを実行
 - ALVR.exeを起動
- ALVR clientをヘッドセットにインストールする方法
 - Gear VR
  - SideloadVR経由でのインストールが楽です
 - Oculus Go
  - release pageからapkをダウンロード
  - adbでapkをインストール

## 今後の予定
- 音声のストリーミングのサポート
- H.265のサポート (現状、H.264のみ)
- Gear VR / Oculus Go コントローラのサポート
- インストーラの作成

## ビルド方法
### ALVR Server and GUI(Launcher)
- ALVR.slnをVisual Studio 2017で開いてビルドします。
 - alvr\_server: C++で書かれたSteamVR (OpenVR) のドライバ
 - ALVR: C#で書かれたGUIでALVR Serverを起動するためのプログラム

### ALVR Client
- [ALVR Client](https://polygraphene.github.com/ALVRClient/)をクローン
- [osig file](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-submission-sig-file/) を assets フォルダに設置 (Gear VRのみ)
- Android Studioでビルド
- adbでインストール

## License
MITライセンスです。
ALVR is licensed under MIT License.

## Donate
If you like this project, please donate!
