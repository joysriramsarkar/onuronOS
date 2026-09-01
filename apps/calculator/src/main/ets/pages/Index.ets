// apps/calculator/src/main/ets/pages/Index.ets
// ArkTS Declarative Calculator App for NilOS

@Entry
@Component
struct CalculatorPage {
  @State expression: string = '০';
  @State result: string = '';
  @State history: Array<string> = [];

  private buttons: Array<Array<string>> = [
    ['C', '( )', '%', '÷'],
    ['৭', '৮', '৯', '×'],
    ['৪', '৫', '৬', '-'],
    ['১', '২', '৩', '+'],
    ['±', '০', '.', '=']
  ];

  handlePress(btn: string) {
    if (btn === 'C') {
      this.expression = '০';
      this.result = '';
    } else if (btn === '=') {
      this.calculate();
    } else {
      if (this.expression === '০') {
        this.expression = btn;
      } else {
        this.expression += btn;
      }
    }
  }

  calculate() {
    // Evaluation logic for standard arithmetic
    this.result = this.expression;
  }

  build() {
    Column() {
      // 1. Header
      Text('ক্যালকুলেটর')
        .fontSize(20)
        .fontWeight(FontWeight.Bold)
        .fontColor('#FFFFFF')
        .margin({ top: 16, bottom: 12 })

      // 2. Display Screen
      Column() {
        Text(this.expression)
          .fontSize(36)
          .fontWeight(FontWeight.Bold)
          .fontColor('#00E5FF')
          .width('100%')
          .textAlign(TextAlign.End)

        if (this.result.length > 0) {
          Text(`= ${this.result}`)
            .fontSize(24)
            .fontColor('#00E676')
            .width('100%')
            .textAlign(TextAlign.End)
            .margin({ top: 8 })
        }
      }
      .width('100%')
      .padding(20)
      .backgroundColor('#141C2B')
      .borderRadius(16)

      Blank()

      // 3. Button Keypad Grid
      Column({ space: 10 }) {
        ForEach(this.buttons, (row: Array<string>) => {
          Row({ space: 10 }) {
            ForEach(row, (btn: string) => {
              Button(btn)
                .width(74)
                .height(64)
                .fontSize(22)
                .fontWeight(FontWeight.Medium)
                .backgroundColor(
                  btn === '=' ? '#00E5FF' :
                  (btn === '÷' || btn === '×' || btn === '-' || btn === '+') ? '#1E3A8A' :
                  (btn === 'C' || btn === '%') ? '#334155' : '#1E293B'
                )
                .fontColor(btn === '=' ? '#0A0E17' : '#FFFFFF')
                .borderRadius(16)
                .onClick(() => {
                  this.handlePress(btn);
                })
            })
          }
        })
      }
      .margin({ bottom: 20 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0A0E17')
    .padding(16)
  }
}
