"""Strict named unittest and PowerShell parser receipts, never native proof."""
import re


def suite(raw,names):
    text=raw.decode('utf-8').replace('\r\n','\n')
    lines=text.splitlines()
    expected=[f'{n} (__main__.Tests.{n}) ... ok' for n in sorted(names)]
    if len(set(names))!=len(names) or not names:raise ValueError('Empty/duplicate expected test inventory')
    if lines[:len(names)]!=expected:raise ValueError('Named pure test results differ')
    tail=lines[len(names):]
    if len(tail)!=5 or tail[0]!='' or tail[1]!='-'*70 or not re.fullmatch(r'Ran '+str(len(names))+r' tests in [0-9]+\.[0-9]+s',tail[2]) or tail[3]!='' or tail[4]!='OK':
        raise ValueError('Strict pure test count/OK trailer differs')
    return {'tests':len(names),'passed':len(names),'skipped':0,'failed':0,'names':sorted(names)}


def powershell(raw):
    if raw.replace(b'\r\n',b'\n')!=b'PS_PARSE_OK\n':raise ValueError('PowerShell syntax witness differs')
    return {'parsed':True,'executed_entry':False}
