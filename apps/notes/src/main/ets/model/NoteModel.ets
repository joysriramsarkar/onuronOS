// apps/notes/src/main/ets/model/NoteModel.ets
// ArkTS Note Data Model and Local Storage Persistence for NilOS

export interface NoteItem {
  id: string;
  title: string;
  content: string;
  category: string;
  updatedAt: string;
  isPinned: boolean;
  colorTag: string;
}

export class NoteManager {
  private static instance: NoteManager;
  private notes: Array<NoteItem> = [];

  private constructor() {
    this.initDefaultNotes();
  }

  public static getInstance(): NoteManager {
    if (!NoteManager.instance) {
      NoteManager.instance = new NoteManager();
    }
    return NoteManager.instance;
  }

  private initDefaultNotes() {
    this.notes = [
      {
        id: '1',
        title: 'নীল ওএস কার্নেল আর্কিটেকচার',
        content: 'মাইক্রোকার্নেল বেসড IPC ও SoftBus ডিস্ট্রিবিউটেড ফ্যাব্রিক প্রস্তুত।\n- zImage বूट ও Ramdisk লোড\n- fscrypt v2 এনক্রিপশন',
        category: 'কাজের নোট',
        updatedAt: '১৬:৩০',
        isPinned: true,
        colorTag: '#00E5FF'
      },
      {
        id: '2',
        title: 'ArkTS অ্যাপ ডেভেলপমেন্ট গাইড',
        content: 'ডিক্লেয়ারেটিভ UI দিয়ে দ্রুত ও স্মুথ অ্যাপ তৈরি করা সম্ভব। ArkCompiler সরাসরি বাইটকোড অপ্টিমাইজ করে।',
        category: 'কোড ও আইডিয়া',
        updatedAt: '১৬:১৫',
        isPinned: true,
        colorTag: '#00E676'
      },
      {
        id: '3',
        title: 'বাজারের তালিকা (Shopping)',
        content: '১. ল্যাপটপ স্ট্যান্ড\n২. টাইপ-সি OTG কেবল\n৩. কফি বিন্স',
        category: 'ব্যক্তিগত',
        updatedAt: '১৫:৫০',
        isPinned: false,
        colorTag: '#FFB300'
      }
    ];
  }

  public getAllNotes(): Array<NoteItem> {
    return this.notes;
  }

  public addNote(title: string, content: string, category: string = 'সাধারণ'): NoteItem {
    const newNote: NoteItem = {
      id: Date.now().toString(),
      title: title || 'শিরোনামহীন নোট',
      content: content || '',
      category: category,
      updatedAt: 'এখন',
      isPinned: false,
      colorTag: '#7C4DFF'
    };
    this.notes.unshift(newNote);
    return newNote;
  }

  public deleteNote(id: string): boolean {
    const index = this.notes.findIndex(n => n.id === id);
    if (index !== -1) {
      this.notes.splice(index, 1);
      return true;
    }
    return false;
  }

  public togglePin(id: string): boolean {
    const note = this.notes.find(n => n.id === id);
    if (note) {
      note.isPinned = !note.isPinned;
      return true;
    }
    return false;
  }
}
