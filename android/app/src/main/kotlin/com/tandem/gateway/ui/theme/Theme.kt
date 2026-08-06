/**
 * Material 3 Compose theme (colors, typography, shapes) for all gateway screens,
 * light and dark. One green accent carries every affirmative state, so the phone
 * and the desktop panel read as the same product.
 */
package com.tandem.gateway.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

private val Accent = Color(0xFF34D399)
private val AccentInk = Color(0xFF00281A)
private val AccentDim = Color(0xFF1B6E4B)

private val DarkColors = darkColorScheme(
    primary = Accent,
    onPrimary = AccentInk,
    primaryContainer = Color(0xFF10362A),
    onPrimaryContainer = Accent,
    secondary = Color(0xFF8AA6B4),
    onSecondary = Color(0xFF0B1418),
    background = Color(0xFF0B0D10),
    onBackground = Color(0xFFECEFF3),
    surface = Color(0xFF11141A),
    onSurface = Color(0xFFECEFF3),
    surfaceVariant = Color(0xFF1A1F27),
    onSurfaceVariant = Color(0xFFA6B0BE),
    outline = Color(0xFF2A313C),
    outlineVariant = Color(0xFF20262F),
    error = Color(0xFFFF6B6B),
    onError = Color(0xFF2A0708),
    errorContainer = Color(0xFF3A1416),
    onErrorContainer = Color(0xFFFFB4AB),
)

private val LightColors = lightColorScheme(
    primary = AccentDim,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFD3F3E3),
    onPrimaryContainer = Color(0xFF04301F),
    secondary = Color(0xFF44606E),
    background = Color(0xFFF7F9FB),
    onBackground = Color(0xFF11151A),
    surface = Color.White,
    onSurface = Color(0xFF11151A),
    surfaceVariant = Color(0xFFEDF1F5),
    onSurfaceVariant = Color(0xFF48525E),
    outline = Color(0xFFD3DAE2),
    outlineVariant = Color(0xFFE4E9EF),
    error = Color(0xFFB3261E),
    onError = Color.White,
)

/**
 * Tighter than the Material defaults: this app is mostly short status lines, and
 * default tracking makes them look loose.
 */
private val TandemTypography = Typography(
    headlineMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 27.sp,
        lineHeight = 33.sp,
        letterSpacing = (-0.5).sp,
    ),
    titleMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        letterSpacing = (-0.1).sp,
    ),
    bodyMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
    ),
    labelSmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 11.sp,
        lineHeight = 14.sp,
        letterSpacing = 0.9.sp,
    ),
)

private val TandemShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

@Composable
fun TandemTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = TandemTypography,
        shapes = TandemShapes,
        content = content,
    )
}
