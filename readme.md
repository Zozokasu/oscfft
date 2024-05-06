# オーディオ入力をスペクトル解析したデータをOSC経由でどっかに送るソフト
- オーディオ入力（マイク）
- 送り先IPアドレス
- 送り先ポート番号

を指定して宛先にオーディオ入力のスペクトル情報を送信できます。一秒間にだいたい20回くらい送信されます。音楽に合わせてスペクトルを表示したいときに使ってください。

見本 https://twitter.com/zozokasu2/status/1786923894328451365
# ダウンロード
ここ https://github.com/Zozokasu/oscfft/releases/tag/release
![image](https://github.com/Zozokasu/oscfft/assets/13605108/1271fde6-16e5-4498-bb4a-11463739eb68)

# OSCアドレス・引数
- /fft/0
- /fft/1
- /fft/2
- /fft/3

の4つのアドレスでそれぞれ256個ずつ、計1024個のfloat型のデータを送信します。

/fft/0の先頭23.4575Hz成分から23.4375Hz刻みのデータが送信されるので、/fft/3の末尾のデータは24000Hzの成分となります。
