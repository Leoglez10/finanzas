import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

private func day(_ y: Int, _ m: Int, _ d: Int, _ h: Int = 0) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d, hour: h))!
}

@MainActor
private final class Store {
    let container: ModelContainer
    let context: ModelContext
    let bank = Account(name: "Banco")
    init() throws {
        container = try FinanzasStore.inMemory()
        context = container.mainContext
        context.insert(bank)
    }
    func schedule(_ name: String, start: Date, next: Date? = nil, autoPost: Bool, completed: Bool = false) -> Schedule {
        let s = Schedule(name: name, amount: -1_000, recurrence: .init(start: start, frequency: .monthly), account: bank,
                         autoPost: autoPost)
        s.nextDate = next
        s.completed = completed
        context.insert(s)
        return s
    }
}

@MainActor @Test func autoPostRecordsEveryDueOccurrenceOfAutomaticSchedulesOnly() throws {
    let st = try Store()
    let auto = st.schedule("iCloud", start: day(2026, 8, 28), autoPost: true)
    let manual = st.schedule("Gym", start: day(2026, 8, 11), autoPost: false)
    let future = st.schedule("Renta", start: day(2026, 10, 20), autoPost: true)
    let done = st.schedule("Viejo", start: day(2026, 1, 1), autoPost: true, completed: true)

    let posted = Automations.postDue([auto, manual, future, done], today: day(2026, 10, 15), in: st.context)

    #expect(posted.map(\.date) == [day(2026, 8, 28), day(2026, 9, 28)])
    #expect(posted.allSatisfy { $0.schedule === auto && $0.amount == -1_000 && $0.account === st.bank })
    #expect(auto.nextDate == day(2026, 10, 28))
    #expect(manual.nextDate == nil && future.nextDate == nil && (done.entries ?? []).isEmpty)
    #expect(Automations.postDue([auto], today: day(2026, 10, 15), in: st.context).isEmpty)  // idempotent
}

@MainActor @Test func remindersFireDaysBeforeAtTheChosenHourAndSkipThePast() throws {
    let st = try Store()
    let gym = st.schedule("Gym", start: day(2026, 10, 11), next: day(2026, 10, 11), autoPost: false)
    let icloud = st.schedule("iCloud", start: day(2026, 10, 28), autoPost: true)
    let items = Upcoming.items(schedules: [gym, icloud], today: day(2026, 10, 15), days: 30)

    let plan = Reminders.plan(items, daysBefore: 2, hour: 9, now: day(2026, 10, 15, 12))

    // Gym is overdue (its reminder time passed); iCloud Oct 28 → Oct 26 09:00; Gym Nov 11 → Nov 9 09:00.
    #expect(plan.map(\.fireDate) == [day(2026, 10, 26, 9), day(2026, 11, 9, 9)])
    #expect(plan[0].title == "Pago próximo: iCloud")
    #expect(plan[0].body == "-$10.00 el 28/10/2026 (Banco) · se registrará automáticamente")
    #expect(plan[1].body == "-$10.00 el 11/11/2026 (Banco)")
    #expect(plan.allSatisfy { $0.id.hasPrefix(Reminders.idPrefix) })
    #expect(plan[0].id == "\(Reminders.idPrefix)\(ModelID.string(icloud.persistentModelID))-20261028")
    #expect(Reminders.plan(items, daysBefore: 2, hour: 9, now: day(2026, 10, 15, 12)).map(\.id) == plan.map(\.id))  // stable
}

@MainActor @Test func payeeSuggestsItsMostRecentCategory() throws {
    let st = try Store()
    let food = Category(name: "Comida"), fun = Category(name: "Salidas")
    let oxxo = Payee(name: "Oxxo"), uber = Payee(name: "Uber")
    [food, fun].forEach { st.context.insert($0) }
    [oxxo, uber].forEach { st.context.insert($0) }
    st.context.insert(LedgerEntry(date: day(2026, 9, 1), amount: -1, account: st.bank, category: fun, payee: oxxo))
    st.context.insert(LedgerEntry(date: day(2026, 9, 20), amount: -1, account: st.bank, category: food, payee: oxxo))
    st.context.insert(LedgerEntry(date: day(2026, 9, 25), amount: -1, account: st.bank, payee: oxxo))  // uncategorized
    try st.context.save()

    #expect(oxxo.suggestedCategory === food)
    #expect(uber.suggestedCategory == nil)

    let blank = LedgerEntry(date: day(2026, 10, 1), amount: -1, account: st.bank, payee: oxxo)
    blank.applyPayeeRule()
    #expect(blank.category === food)
    let explicit = LedgerEntry(date: day(2026, 10, 1), amount: -1, account: st.bank, category: fun, payee: oxxo)
    explicit.applyPayeeRule()
    #expect(explicit.category === fun)
}

@MainActor @Test func postedSchedulesUseThePayeeCategoryWhenTheyHaveNone() throws {
    let st = try Store()
    let platforms = Category(name: "Plataformas"), apple = Payee(name: "Apple")
    st.context.insert(platforms); st.context.insert(apple)
    st.context.insert(LedgerEntry(date: day(2026, 8, 28), amount: -4_900, account: st.bank, category: platforms, payee: apple))
    let icloud = st.schedule("iCloud", start: day(2026, 9, 28), autoPost: true)
    icloud.payee = apple
    try st.context.save()

    let posted = Automations.postDue([icloud], today: day(2026, 10, 15), in: st.context)
    #expect(posted.map { $0.category?.name } == ["Plataformas"])
}
