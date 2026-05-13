import VideoEditorApp from "@/features/video/components/VideoEditorApp";
import AuthRequiredGate from "@/app/components/AuthRequiredGate";

export default function VideoPage() {
  return (
    <AuthRequiredGate>
      <VideoEditorApp />
    </AuthRequiredGate>
  );
}
