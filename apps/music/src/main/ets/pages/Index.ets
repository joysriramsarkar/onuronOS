// apps/music/src/main/ets/pages/Index.ets
// ArkTS Declarative Music Player for NilOS

@Entry
@Component
struct MusicPlayerPage {
  @State isPlaying: boolean = true;
  @State currentTrack: string = 'মেঘের কোলে রোদ হেসেছে';
  @State artist: string = 'রবীন্দ্রনাথ ঠাকুর';
  @State progress: number = 42; // percent

  private playlist: Array<{ title: string; artist: string; duration: string }> = [
    { title: 'মেঘের কোলে রোদ হেসেছে', artist: 'রবীন্দ্রনাথ ঠাকুর', duration: '৩:৪৫' },
    { title: 'আগুনের পরশমণি', artist: 'রবীন্দ্রনাথ ঠাকুর', duration: '৪:১২' },
    { title: 'ধনধান্য পুষ্পভরা', artist: 'দ্বিজেন্দ্রলাল রায়', duration: '৩:৩০' },
    { title: 'কারার ঐ লৌহকপাট', artist: 'কাজী নজরুল ইসলাম', duration: '৩:১৫' },
  ];

  build() {
    Column() {
      // Header
      Row() {
        Text('সঙ্গীত ও অডিও')
          .fontSize(20)
          .fontWeight(FontWeight.Bold)
          .fontColor('#FFFFFF')
      }
      .width('100%')
      .padding({ top: 16, bottom: 12 })

      // Album Art Card
      Column() {
        Text('🎵')
          .fontSize(64)
          .margin({ top: 40, bottom: 20 })

        Text(this.currentTrack)
          .fontSize(18)
          .fontWeight(FontWeight.Bold)
          .fontColor('#00E5FF')

        Text(this.artist)
          .fontSize(14)
          .fontColor('#94A3B8')
          .margin({ top: 4, bottom: 24 })
      }
      .width('100%')
      .backgroundColor('#141C2B')
      .borderRadius(20)
      .alignItems(HorizontalAlign.Center)

      // Playlist
      Text('প্লেলিস্ট')
        .fontSize(16)
        .fontWeight(FontWeight.Bold)
        .fontColor('#FFFFFF')
        .width('100%')
        .margin({ top: 20, bottom: 8 })

      List({ space: 8 }) {
        ForEach(this.playlist, (item) => {
          ListItem() {
            Row() {
              Column() {
                Text(item.title)
                  .fontSize(14)
                  .fontColor('#FFFFFF')
                Text(item.artist)
                  .fontSize(12)
                  .fontColor('#94A3B8')
              }
              .alignItems(HorizontalAlign.Start)

              Blank()

              Text(item.duration)
                .fontSize(12)
                .fontColor('#475569')
            }
            .width('100%')
            .padding(12)
            .backgroundColor('#1E293B')
            .borderRadius(12)
          }
        })
      }
      .layoutWeight(1)
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0A0E17')
    .padding(16)
  }
}
