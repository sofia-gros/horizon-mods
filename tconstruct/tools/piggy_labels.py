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

def mark_unavailable(language, code):
    warning = WARNINGS[code]
    key = 'item.tconstruct.piggy_backpack'
    language[key] = language.get(key, 'Piggybackpack') + f' ({warning})'
    language['item.tconstruct.piggy_backpack.tooltip'] = warning
    return language
