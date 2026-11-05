"""Temporary, explicit user-facing limitation for the partial backpack item."""
WARNINGS = {
    'de_de': 'Mob-Tragen noch nicht verfügbar',
    'en_us': 'mob carrying unavailable',
    'es_ar': 'transporte de criaturas no disponible',
    'es_cl': 'transporte de criaturas no disponible',
    'es_es': 'transporte de criaturas no disponible',
    'es_mx': 'transporte de criaturas no disponible',
    'fr_fr': 'transport de créatures indisponible',
    'id_id': 'menggendong mob belum tersedia',
    'it_it': 'trasporto delle creature non disponibile',
    'ja_jp': 'Mobの運搬は未対応',
    'ko_kr': '몹 운반 미지원',
    'lzh': '未能負生物',
    'pl_pl': 'przenoszenie mobów jest niedostępne',
    'pt_br': 'transporte de criaturas indisponível',
    'ru_ru': 'перенос существ недоступен',
    'sv_se': 'det går inte att bära varelser än',
    'tr_tr': 'yaratık taşıma henüz kullanılamıyor',
    'uk_ua': 'перенесення істот недоступне',
    'zh_cn': '暂不支持搬运生物',
    'zh_tw': '尚不支援搬運生物',
}

# Gold Platform can be used as a Basin cast and crafting ingredient, but the
# block's connection and placement behavior is not implemented yet.
PLATFORM_WARNINGS = {
    'de_de': 'Platzieren nicht verfügbar',
    'en_us': 'placement unavailable',
    'es_ar': 'colocación no disponible', 'es_cl': 'colocación no disponible',
    'es_es': 'colocación no disponible', 'es_mx': 'colocación no disponible',
    'fr_fr': 'placement indisponible', 'id_id': 'penempatan belum tersedia',
    'it_it': 'posizionamento non disponibile', 'ja_jp': '設置は未対応',
    'ko_kr': '설치 미지원', 'lzh': '未能置放',
    'pl_pl': 'stawianie niedostępne', 'pt_br': 'posicionamento indisponível',
    'ru_ru': 'установка недоступна', 'sv_se': 'kan inte placeras',
    'tr_tr': 'yerleştirme kullanılamıyor', 'uk_ua': 'розміщення недоступне',
    'zh_cn': '暂不支持放置', 'zh_tw': '尚不支援放置',
}

MACHINE_GUARD_MESSAGES = {
    'en_us': 'Cannot break: check machine contents or unsupported saved data.',
    'ja_jp': '破壊できません。機械の中身または未対応の保存データを確認してください。',
    'de_de': 'Abbau nicht möglich: Maschineninhalt oder nicht unterstützte Speicherdaten prüfen.',
    'es_ar': 'No se puede romper: revisa el contenido o los datos guardados no compatibles.',
    'es_cl': 'No se puede romper: revisa el contenido o los datos guardados no compatibles.',
    'es_es': 'No se puede romper: revisa el contenido o los datos guardados no compatibles.',
    'es_mx': 'No se puede romper: revisa el contenido o los datos guardados no compatibles.',
    'fr_fr': 'Destruction impossible : vérifiez le contenu ou les données sauvegardées non prises en charge.',
    'id_id': 'Tidak dapat dihancurkan: periksa isi mesin atau data tersimpan yang tidak didukung.',
    'it_it': 'Impossibile rompere: controlla il contenuto o i dati salvati non supportati.',
    'ko_kr': '파괴할 수 없습니다. 기계 내용물이나 지원되지 않는 저장 데이터를 확인하세요.',
    'lzh': '不可毀：請察機中之物或未支援之存檔。',
    'pl_pl': 'Nie można zniszczyć: sprawdź zawartość lub nieobsługiwane dane zapisu.',
    'pt_br': 'Não é possível quebrar: verifique o conteúdo ou os dados salvos não suportados.',
    'ru_ru': 'Нельзя сломать: проверьте содержимое или неподдерживаемые данные сохранения.',
    'sv_se': 'Kan inte förstöras: kontrollera innehåll eller sparad data som inte stöds.',
    'tr_tr': 'Kırılamıyor: makine içeriğini veya desteklenmeyen kayıt verilerini kontrol edin.',
    'uk_ua': 'Неможливо зламати: перевірте вміст або непідтримувані дані збереження.',
    'zh_cn': '无法破坏：请检查机器内容或不支持的存档数据。',
    'zh_tw': '無法破壞：請檢查機器內容或不支援的存檔資料。',
}

def mark_unavailable(language, code):
    warning = WARNINGS[code]
    key = 'item.tconstruct.piggy_backpack'
    language[key] = language.get(key, 'Piggybackpack') + f' ({warning})'
    language['item.tconstruct.piggy_backpack.tooltip'] = warning
    platform = language.get('block.tconstruct.gold_platform', 'Gold Platform')
    language['item.tconstruct.gold_platform'] = platform + f" ({PLATFORM_WARNINGS[code]})"
    language['message.tconstruct.machine_break_guard'] = MACHINE_GUARD_MESSAGES[code]
    return language
