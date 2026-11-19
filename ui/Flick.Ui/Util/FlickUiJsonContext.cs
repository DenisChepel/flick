using System.Text.Json.Serialization;

namespace Flick.Util;

[JsonSerializable(typeof(SessionInfo))]
[JsonSerializable(typeof(SendArgsRequestModel))]
[JsonSerializable(typeof(Flick.UI.Pages.CreditsData))]
[JsonSerializable(typeof(Flick.UI.RecorderSettingsJson))]
internal partial class FlickUiJsonContext : JsonSerializerContext { }
