# Writes tests/fixtures/estado.pdf: python3 tests/fixtures/mkpdf.py tests/fixtures/estado.pdf
# Minimal PDF where every cell is its own positioned text object, like bank statements.
import sys
rows = [["Banco Ejemplo - Estado de cuenta"], ["Periodo: 01/11/2026 al 30/11/2026"],
        ["FECHA", "DESCRIPCION", "CARGOS", "ABONOS", "SALDO"],
        ["03/NOV", "COMPRA OXXO CENTRO 1234", "45.50", "", "9,954.50"],
        ["05/NOV", "DEPOSITO NOMINA", "", "15,000.00", "24,954.50"],
        ["07/NOV", "PAGO SERVICIO", "500.00", "", "24,454.50"],
        ["", "TELMEX HOGAR", "", "", ""],
        ["", "SALDO FINAL", "", "", "24,454.50"]]
xs = [40, 110, 330, 420, 510]
ops = ["BT /F1 9 Tf"]
y = 760
for r in rows:
    for i, cell in enumerate(r):
        if cell: ops.append(f"1 0 0 1 {xs[i]} {y} Tm ({cell}) Tj")
    y -= 16
ops.append("ET")
content = "\n".join(ops).encode()
objs = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"]
out = b"%PDF-1.4\n"; offs = []
for i, o in enumerate(objs, 1):
    offs.append(len(out)); out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
x = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offs)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
open(sys.argv[1], "wb").write(out)
