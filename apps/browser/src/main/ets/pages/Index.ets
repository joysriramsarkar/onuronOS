// apps/browser/src/main/ets/pages/Index.ets
// ArkTS Declarative Web Browser for NilOS (Firefox Engine Bridge)

@Entry
@Component
struct BrowserPage {
  @State urlInput: string = 'https://nilos.dev';
  @State currentTitle: string = 'NilOS — নেক্সট-জেন বাংলা অপারেটিং সিস্টেম';
  @State isLoading: boolean = false;
  @State searchEngine: string = 'DuckDuckGo';

  private bookmarks: Array<{ title: string; url: string; icon: string }> = [
    { title: 'নীল ওএস পোর্টাল', url: 'https://nilos.dev', icon: '🌐' },
    { title: 'উইকিপিডিয়া বাংলা', url: 'https://bn.wikipedia.org', icon: '📚' },
    { title: 'গুগল অনুসন্ধান', url: 'https://google.com', icon: '🔍' },
    { title: 'গিটহাব কোড', url: 'https://github.com', icon: '💻' },
    { title: 'আনন্দবাজার পত্রিকা', url: 'https://anandabazar.com', icon: '📰' },
  ];

  build() {
    Column() {
      // 1. Navigation / URL Bar
      Row({ space: 8 }) {
        Button('<')
          .width(36)
          .height(36)
          .fontSize(16)
          .backgroundColor('#141C2B')
          .fontColor('#FFFFFF')
          .borderRadius(18)

        Button('>')
          .width(36)
          .height(36)
          .fontSize(16)
          .backgroundColor('#141C2B')
          .fontColor('#FFFFFF')
          .borderRadius(18)

        TextInput({ text: this.urlInput, placeholder: 'ওয়েব ঠিকানা বা সার্চ লিখুন...' })
          .layoutWeight(1)
          .height(38)
          .fontSize(13)
          .fontColor('#00E5FF')
          .backgroundColor('#141C2B')
          .borderRadius(12)
          .onChange((val: string) => {
            this.urlInput = val;
          })

        Button('⟳')
          .width(36)
          .height(36)
          .fontSize(16)
          .backgroundColor('#00E5FF')
          .fontColor('#0A0E17')
          .borderRadius(18)
      }
      .width('100%')
      .padding({ left: 12, right: 12, top: 10, bottom: 8 })

      // 2. Bookmarks Quick Dial Bar
      Row({ space: 8 }) {
        ForEach(this.bookmarks, (bm) => {
          Row({ space: 4 }) {
            Text(bm.icon).fontSize(14)
            Text(bm.title).fontSize(12).fontColor('#94A3B8')
          }
          .padding({ left: 10, right: 10, top: 6, bottom: 6 })
          .backgroundColor('#1E293B')
          .borderRadius(14)
          .onClick(() => {
            this.urlInput = bm.url;
            this.currentTitle = bm.title;
          })
        })
      }
      .width('100%')
      .padding({ left: 12, right: 12, bottom: 10 })

      // 3. Web View Content Canvas
      Column() {
        Row() {
          Text('🔒 সুরক্ষিত সংযোগ (SSL / TLS 1.3)')
            .fontSize(11)
            .fontColor('#00E676')
          Blank()
          Text('প্রাইভেট শিল্ড সক্রিয়')
            .fontSize(11)
            .fontColor('#00E5FF')
        }
        .width('100%')
        .padding({ left: 16, right: 16, top: 12, bottom: 8 })

        Divider().color('#1E293B')

        Column() {
          Text(this.currentTitle)
            .fontSize(20)
            .fontWeight(FontWeight.Bold)
            .fontColor('#FFFFFF')
            .margin({ top: 16, bottom: 8 })

          Text('নীল ওএস (NilOS) একটি সম্পূর্ণ আধুনিক, দ্রুতগতির এবং ওপেন সোর্স মোবাইল ও ডেস্কটপ অপারেটিং সিস্টেম। এটি মাইক্রোকার্নেল আর্কিটেকচার, SoftBus ডিস্ট্রিবিউটেড মেশ নেটওয়ার্ক এবং ArkTS ডিক্লেয়ারেটিভ UI দিয়ে চালিত।')
            .fontSize(14)
            .fontColor('#94A3B8')
            .lineHeight(22)

          Text('প্রধান বৈশিষ্ট্যসমূহ:')
            .fontSize(16)
            .fontWeight(FontWeight.Bold)
            .fontColor('#00E5FF')
            .margin({ top: 16, bottom: 8 })

          Text('• লিনাক্স GNU ব্যাশ টার্মিনাল ও ন্যানো এডিটর\n• HarfBuzz ভেক্টর বাংলা টাইপোগ্রাফি\n• ডায়নামিক আইল্যান্ড ও গ্লাস মরফিজম\n• AOSP অ্যান্ড্রয়েড কন্টেইনার সাপোর্ট')
            .fontSize(13)
            .fontColor('#E2E8F0')
            .lineHeight(20)
        }
        .width('100%')
        .padding(16)
        .alignItems(HorizontalAlign.Start)
      }
      .layoutWeight(1)
      .width('100%')
      .backgroundColor('#0D131F')
      .borderRadius(16)
      .margin({ left: 12, right: 12, bottom: 12 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0A0E17')
  }
}
