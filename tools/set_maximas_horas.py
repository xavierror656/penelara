"""Make a controlled .xghc variant of consecutive-hour limits."""

import argparse
import gzip
import re
from pathlib import Path


def replace_limit(xml, section, value):
    pattern = rb"(<" + section + rb">[\s\S]*?<maximasHorasSeguidas(?:\s+[^>]*)?>)(\d+)(</maximasHorasSeguidas>)"
    updated, count = re.subn(pattern, lambda m: m[1] + str(value).encode() + m[3], xml, count=1)
    if count != 1:
        raise ValueError(f"No se encontró un límite en {section.decode()}")
    return updated


def replace_teacher_limit(xml, teacher_index, value):
    teachers = list(re.finditer(rb"<profesor>.*?</profesor>", xml, re.DOTALL))
    if teacher_index < 0 or teacher_index >= len(teachers):
        raise ValueError(f"profesor {teacher_index} fuera de 0..{len(teachers) - 1}")
    teacher = teachers[teacher_index]
    block, count = re.subn(
        rb"(<maximasHorasSeguidas(?:\s+[^>]*)?>)(\d+)(</maximasHorasSeguidas>)",
        lambda m: m[1] + str(value).encode() + m[3],
        teacher.group(), count=1,
    )
    if count != 1:
        raise ValueError(f"profesor {teacher_index} sin maximasHorasSeguidas")
    return xml[:teacher.start()] + block + xml[teacher.end():]


def omit_first_teacher_limit(xml):
    pattern = rb"(<profesor>[\s\S]*?)<maximasHorasSeguidas(?:\s+[^>]*)?>\d+</maximasHorasSeguidas>"
    updated, count = re.subn(pattern, lambda m: m[1], xml, count=1)
    if count != 1:
        raise ValueError("No se encontró el límite del primer profesor")
    return updated


def set_limit_strict(xml, section, strict):
    pattern = rb"(<" + section + rb">[\s\S]*?)<maximasHorasSeguidas>(\d+)</maximasHorasSeguidas>"
    updated, count = re.subn(
        pattern,
        lambda m: m[1] + b'<maximasHorasSeguidas estricto="' + strict.encode() + b'">' + m[2] + b"</maximasHorasSeguidas>",
        xml,
        count=1,
    )
    if count != 1:
        raise ValueError(f"No se encontró el límite sin atributo en {section.decode()}")
    return updated


def move_otros_before_teachers(xml):
    otros = re.search(rb"(?m)^\t<otros>\n[\s\S]*?^\t</otros>\n", xml)
    teachers = re.search(rb"(?m)^\t<profesores>\n", xml)
    if otros is None or teachers is None or otros.start() <= teachers.start():
        raise ValueError("orden de secciones inesperado")
    return xml[:teachers.start()] + otros.group() + xml[teachers.start():otros.start()] + xml[otros.end():]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--global-limit", type=int)
    parser.add_argument("--first-teacher-limit", "--teacher-limit", dest="first_teacher_limit", type=int)
    parser.add_argument("--teacher-index", type=int, default=0)
    parser.add_argument("--omit-first-teacher-limit", action="store_true")
    parser.add_argument("--first-teacher-strict", action="store_true")
    parser.add_argument("--first-teacher-strict-false", action="store_true")
    parser.add_argument("--global-strict", action="store_true")
    parser.add_argument("--move-otros-before-teachers", action="store_true")
    args = parser.parse_args()
    if args.first_teacher_limit is not None and args.omit_first_teacher_limit:
        parser.error("elige cambiar u omitir el límite individual")
    if args.first_teacher_strict and args.omit_first_teacher_limit:
        parser.error("no puede ser estricto un límite omitido")
    if args.first_teacher_strict_false and args.omit_first_teacher_limit:
        parser.error("no puede tener atributo un límite omitido")
    if args.first_teacher_strict and args.first_teacher_strict_false:
        parser.error("elige un valor para estricto del primer profesor")
    if args.global_limit is None and args.first_teacher_limit is None and not args.omit_first_teacher_limit and not args.first_teacher_strict and not args.first_teacher_strict_false and not args.global_strict and not args.move_otros_before_teachers:
        parser.error("indica al menos un límite")
    for value in (args.global_limit, args.first_teacher_limit):
        if value is not None and not 1 <= value <= 14:
            parser.error("los límites nativos válidos están entre 1 y 14")
    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    if start != 7 or payload[0] != ord("M") or payload[end:end + 1] != b"\n":
        raise ValueError("cabecera o final XML inesperados")
    xml_end = end + 1
    if int.from_bytes(payload[3:7], "little") != xml_end - start:
        raise ValueError("longitud XML nativa incorrecta")
    xml = payload[start:xml_end]
    if args.first_teacher_limit is not None:
        xml = replace_teacher_limit(xml, args.teacher_index, args.first_teacher_limit)
    elif args.omit_first_teacher_limit:
        xml = omit_first_teacher_limit(xml)
    if args.first_teacher_strict:
        xml = set_limit_strict(xml, b"profesor", "true")
    elif args.first_teacher_strict_false:
        xml = set_limit_strict(xml, b"profesor", "false")
    if args.global_limit is not None:
        xml = replace_limit(xml, b"valorInicialProfesores", args.global_limit)
    if args.global_strict:
        xml = set_limit_strict(xml, b"valorInicialProfesores", "true")
    if args.move_otros_before_teachers:
        xml = move_otros_before_teachers(xml)
    result = bytearray(payload[:start] + xml + payload[xml_end:])
    result[3:7] = len(xml).to_bytes(4, "little")
    packed = bytearray(gzip.compress(result, mtime=0))
    packed[9] = 10
    args.destination.write_bytes(packed)
    print(f"{args.destination}: global={args.global_limit}, profesor[{args.teacher_index}]={args.first_teacher_limit}, omitido={args.omit_first_teacher_limit}, estricto={args.first_teacher_strict}, estricto_false={args.first_teacher_strict_false}, global_estricto={args.global_strict}, otros_antes={args.move_otros_before_teachers}")


if __name__ == "__main__":
    main()
