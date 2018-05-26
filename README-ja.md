# ALVR - Air Light VR

ALVRはPCVRの画面をGear VRやOculus Goに転送して遊ぶためのソフトです。SteamVRのゲームをスタンドアロン型のヘッドセットでプレイすることができます。

[English](https://github.com/polygraphene/ALVR/) Japanese

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
- SteamVRがインストール済みであること

## インストール方法
- ALVR serverのインストールする方法
    - [Releases](https://github.com/polygraphene/ALVR/releases)からzipをダウンロード
    - 任意のフォルダに展開
    - driverフォルダ内のdriver\_install.batを実行
    - ALVR.exeを起動
- ALVR clientをヘッドセットにインストールする方法
    - Gear VR
        - SideloadVRにリリース予定 (審査が通れば)
        - [Releases](https://github.com/polygraphene/ALVR/releases)からapkをダウンロード
        - [Apk Editor](https://play.google.com/store/apps/details?id=com.gmail.heagoo.apkeditor)等でapkのassetsフォルダにosigファイルを置く
        - apkを署名(Apk EditorならBuild)してインストール
    - Oculus Go
        - [Releases](https://github.com/polygraphene/ALVR/releases)からapkをダウンロード
        - adbでapkをインストール

## 使い方
- SteamVRをインストールする
- ALVR.exeを起動
- Start Serverボタンを押す or VR対応ゲームを起動
- SteamVRの小さいウィンドウが出てくる
- ヘッドセットでALVR Clientを起動
- ALVR.exeの画面にヘッドセットのIPアドレスが出てくるのでConnectを押す

## アンインストール方法
- driverフォルダ内のdriver\_uninstall.batを実行
- インストールフォルダを削除 (レジストリは使いません)
- driver\_uninstall.batを実行せず削除してしまった場合
    - C:\Users\\%USERNAME%\AppData\Local\openvr\openvrpaths.vrpathをメモ帳で開きインストールフォルダを確認(手動で書き換えしないように)
    - コマンドプロンプトで
    `"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver (インストールフォルダ)`
    を実行

## 今後の予定
- 動画ビットレートの変更機能
- 音声のストリーミングのサポート
- H.265のサポート (現状、H.264のみ)
- Gear VR / Oculus Go コントローラのサポート
- インストーラの作成

## ビルド方法
### ALVR Server and GUI(Launcher)
- ALVR.slnをVisual Studio 2017で開いてビルドします。
    - alvr\_server: SteamVR (OpenVR) のドライバ (C++)
    - ALVR: ALVR Serverを起動/制御するためのGUI (C#)

### ALVR Client
- [ALVR Client](https://github.com/polygraphene/ALVRClient)をクローン
- [osig file](https://developer.oculus.com/documentation/mobilesdk/latest/concepts/mobile-submission-sig-file/) を assets フォルダに設置 (Gear VRのみ)
- Android Studioでビルド
- adbでインストール

## License
MITライセンスです。
ALVR is licensed under MIT License.

## Donate
If you like this project, please donate!
