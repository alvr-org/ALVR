# ALVR - Air Light VR

ALVRはPCVRの画面をGear VRやOculus Goに転送して遊ぶためのソフトです。SteamVRのゲームをスタンドアロン型のヘッドセットでプレイすることができます。

[English](https://github.com/polygraphene/ALVR/) Japanese

## 特徴
ALVRはPCVRの画面をエンコードしWi-Fi経由でヘッドセットに転送します。似たソフトとしてRiftcatやTrinus VRがありますが、それらよりもGear VR向けに特化しているのが特徴です。Wi-Fi経由でもGear VRに搭載されたAsynchronous Timewarpを利用してスムーズなヘッドトラッキングを実現できます。

Gear VR / Oculus Go のコントローラーをサポートしました！

注意：PCVRでは6DoFコントローラーや多数のボタンが必要なゲームもあり、プレイできないことも多々あります。

## 動作環境
以下の動作環境が必要です。
- Gear VR または Oculus Go

|機種|動作確認|
|---|---|
|Oculus Go|OK|
|GalaxyS9/S9+|OK|
|GalaxyS8/S8+|OK|
|GalaxyS7|OK|
|GalaxyS6(Edge)|OK|

- NVENCが使えるNVIDIA GPUを搭載したハイエンドPC
    - 現在、Windows 10のみサポートしています。
- 802.11n/ac Wi-Fi または 有線接続
    - PCは有線、ヘッドセットは無線がおすすめです (同じルータに接続していればOK)
- SteamVRがインストール済みであること

## 動作確認済みゲーム

- @Thaurinさんが[動作確認済みのゲーム一覧](https://github.com/polygraphene/ALVR/wiki/List-of-tested-VR-games-and-experiences) を作成してくれました！ありがとうございます！
    - 誰でも編集できるので追加なども歓迎です！
    
## インストール方法
### ALVR serverのインストールする方法
- SteamVRをインストール
- [ここ](https://www.microsoft.com/en-us/download/details.aspx?id=53840)からvc\_redist.x64.exeをダウンロードしてインストール
- [Releases](https://github.com/polygraphene/ALVR/releases)からzipをダウンロード
- 任意のフォルダに展開
- ALVR.exeを起動
### ALVR clientをヘッドセットにインストールする方法
#### Gear VR
- SideloadVRにリリース予定 (審査が通れば)
- [Releases](https://github.com/polygraphene/ALVR/releases)からapkをダウンロード
- [Apk Editor](https://play.google.com/store/apps/details?id=com.gmail.heagoo.apkeditor)等でapkのassetsフォルダにosigファイルを置く
- apkを署名(Apk EditorならBuild)してインストール
#### Oculus Go
- [Releases](https://github.com/polygraphene/ALVR/releases)からapkをダウンロード
- adbでapkをインストール

## 使い方
- ALVR.exeを起動
- Start Serverボタンを押す or VR対応ゲームを起動
- SteamVRの小さいウィンドウが出てくる
- ヘッドセットでALVR Clientを起動
- ALVR.exeの画面にヘッドセットのIPアドレスが出てくるのでConnectを押す

## トラブルシューティング
- "Start server"を押しても、"Server is down" と表示され続ける場合
    - driverフォルダのdriver\_install.batをもう一度実行してみる
    - タスクマネージャでvrserver.exeを強制終了してみる
- 画面にヘッドセットのIPアドレスが出てこない場合
    - おそらくネットワーク周りの問題
    - PCとヘッドセットが同じLAN(同じルータ)につながっているか確認
    - ファイアウォールの設定を確認する (UDP/9944番ポートが許可されているか)
    - adbが使える場合、`adb shell ping -c 5 (PCのIPアドレス)`を実行してpingが成功するか確認
- ストリーミングの品質が悪い場合 (よく止まる、カクカクする、画面が乱れる)
    - 解像度やビットレート、バッファサイズを変更してみる(変更後はサーバの再起動が必要)
    - 可能なら5GHzの802.11acの無線LANを使用する or ヘッドセットを有線LANで接続する
- "SteamVRの主要コンポーネントが正しく動作していません。"と表示される場合
    - NVIDIAのグラフィックドライバを最新版にアップデートしてみてください

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

#### Donate by paypal
[![Donate](https://img.shields.io/badge/Donate-PayPal-green.svg)](https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=polygraphene@gmail.com&lc=US&item_name=Donate+for+ALVR+developer&no_note=0&cn=&curency_code=USD&bn=PP-DonationsBF:btn_donateCC_LG.gif:NonHosted)
もしうまくいかない場合は以下の手順をお試しください。
1. PayPalにログイン
2. "支払いと請求" タブを開く
3. "商品またはサービスの代金を支払う" をクリック
4. メールアドレスに "polygraphene@gmail.com" (作者のPayPalアカウントです) を入力

#### Donate by bitcoin
bitcoin:1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv
