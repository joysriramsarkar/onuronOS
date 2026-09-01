// apps/notes/src/main/ets/pages/Index.ets
// ArkTS Declarative Notes App for NilOS / OpenHarmony

import { NoteItem, NoteManager } from '../model/NoteModel';

@Entry
@Component
struct NotesHomePage {
  @State noteList: Array<NoteItem> = [];
  @State selectedCategory: string = 'সব নোট';
  @State searchQuery: string = '';
  @State isCreating: boolean = false;
  @State newTitle: string = '';
  @State newContent: string = '';

  private noteManager: NoteManager = NoteManager.getInstance();
  private categories: Array<string> = ['সব নোট', 'কাজের নোট', 'কোড ও আইডিয়া', 'ব্যক্তিগত'];

  aboutToAppear() {
    this.refreshNotes();
  }

  refreshNotes() {
    this.noteList = this.noteManager.getAllNotes();
  }

  getFilteredNotes(): Array<NoteItem> {
    return this.noteList.filter(note => {
      const matchCategory = (this.selectedCategory === 'সব নোট') || (note.category === this.selectedCategory);
      const matchSearch = this.searchQuery === '' || 
        note.title.toLowerCase().includes(this.searchQuery.toLowerCase()) ||
        note.content.toLowerCase().includes(this.searchQuery.toLowerCase());
      return matchCategory && matchSearch;
    });
  }

  build() {
    Column() {
      // 1. Top Header
      Row() {
        Column() {
          Text('স্মার্ট নোটস')
            .fontSize(22)
            .fontWeight(FontWeight.Bold)
            .fontColor('#FFFFFF')
          Text(`${this.noteList.length}টি সংরক্ষিত নোট`)
            .fontSize(12)
            .fontColor('#94A3B8')
        }
        .alignItems(HorizontalAlign.Start)

        Blank()

        Button('+ নতুন নোট')
          .backgroundColor('#00E5FF')
          .fontColor('#0A0E17')
          .fontSize(13)
          .fontWeight(FontWeight.Medium)
          .borderRadius(18)
          .height(36)
          .onClick(() => {
            this.isCreating = true;
          })
      }
      .width('100%')
      .padding({ left: 16, right: 16, top: 12, bottom: 8 })

      // 2. Search Box
      Search({ value: this.searchQuery, placeholder: 'নোট অনুসন্ধান করুন...' })
        .placeholderColor('#475569')
        .textFont({ size: 14 })
        .backgroundColor('#141C2B')
        .borderRadius(12)
        .height(40)
        .margin({ left: 16, right: 16, top: 6, bottom: 10 })
        .onChange((value: string) => {
          this.searchQuery = value;
        })

      // 3. Category Filter Chips
      Row({ space: 8 }) {
        ForEach(this.categories, (cat: string) => {
          Text(cat)
            .fontSize(13)
            .fontColor(this.selectedCategory === cat ? '#0A0E17' : '#94A3B8')
            .backgroundColor(this.selectedCategory === cat ? '#00E5FF' : '#1E293B')
            .padding({ left: 12, right: 12, top: 6, bottom: 6 })
            .borderRadius(14)
            .onClick(() => {
              this.selectedCategory = cat;
            })
        })
      }
      .width('100%')
      .padding({ left: 16, right: 16, bottom: 12 })

      // 4. Notes List
      List({ space: 10 }) {
        ForEach(this.getFilteredNotes(), (item: NoteItem) => {
          ListItem() {
            Column() {
              Row() {
                Text(item.title)
                  .fontSize(16)
                  .fontWeight(FontWeight.Bold)
                  .fontColor('#FFFFFF')
                  .maxLines(1)
                
                Blank()

                if (item.isPinned) {
                  Text('📌')
                    .fontSize(12)
                }
              }
              .width('100%')

              Text(item.content)
                .fontSize(13)
                .fontColor('#94A3B8')
                .maxLines(2)
                .margin({ top: 4, bottom: 8 })

              Row() {
                Text(item.category)
                  .fontSize(11)
                  .fontColor(item.colorTag)
                  .backgroundColor('#0B3C5D')
                  .padding({ left: 6, right: 6, top: 2, bottom: 2 })
                  .borderRadius(6)

                Blank()

                Text(item.updatedAt)
                  .fontSize(11)
                  .fontColor('#475569')
              }
              .width('100%')
            }
            .padding(14)
            .backgroundColor('#141C2B')
            .borderRadius(14)
            .border({ width: 1, color: '#1E293B' })
            .width('100%')
          }
        }, (item: NoteItem) => item.id)
      }
      .layoutWeight(1)
      .width('100%')
      .padding({ left: 16, right: 16 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0A0E17')
  }
}
